use std::collections::HashSet;

use crate::{
    analysis::{diagnostics::Diagnostic, functions::FunctionKind, validate::Validator},
    ir::{
        bound::{BoundExpr, BoundSelect, BoundStatement, BoundTableSource},
        ids::{ColumnId, ExprId},
    },
};

#[derive(Default)]
pub(crate) struct ExprAnalysis {
    pub(crate) columns: HashSet<ColumnId>,
    pub(crate) has_aggregate: bool,
    pub(crate) has_window: bool,
}

impl ExprAnalysis {
    pub(crate) fn has_grouping_function(&self) -> bool {
        self.has_aggregate || self.has_window
    }
}

impl Validator {
    pub(super) fn validate_grouping(&mut self, stmt: &BoundStatement, select: &BoundSelect) {
        let has_group_by = !select.group_by.is_empty();

        let mut group_columns = HashSet::new();
        for expr_id in &select.group_by {
            let analysis = analyze_group_expr(stmt, *expr_id);
            group_columns.extend(analysis.columns);
        }

        let mut has_aggregate = false;
        for proj in &select.projection {
            if analyze_group_expr(stmt, proj.expr).has_aggregate {
                has_aggregate = true;
            }
        }
        if let Some(having) = select.having {
            if analyze_group_expr(stmt, having).has_aggregate {
                has_aggregate = true;
            }
        }

        let require_grouping = has_group_by || has_aggregate;
        if !require_grouping {
            return;
        }

        if self.dialect != sqlex_common::dialect::Dialect::SQLite {
            for proj in &select.projection {
                let analysis = analyze_group_expr(stmt, proj.expr);
                if analysis.has_grouping_function() {
                    continue;
                }
                if !analysis.columns.is_subset(&group_columns) {
                    let missing = analysis
                        .columns
                        .difference(&group_columns)
                        .map(|col| format_column_ref(stmt, *col))
                        .collect::<Vec<_>>();
                    self.diagnostics.push(Diagnostic::grouping_error(format!(
                        "Non-aggregated SELECT columns must appear in GROUP BY: {}",
                        missing.join(", ")
                    )));
                }
            }
        }

        if self.dialect != sqlex_common::dialect::Dialect::SQLite {
            if let Some(having) = select.having {
                let analysis = analyze_group_expr(stmt, having);
                if !analysis.has_grouping_function() && !analysis.columns.is_subset(&group_columns)
                {
                    let missing = analysis
                        .columns
                        .difference(&group_columns)
                        .map(|col| format_column_ref(stmt, *col))
                        .collect::<Vec<_>>();
                    self.diagnostics.push(Diagnostic::grouping_error(format!(
                        "HAVING references non-grouped columns: {}",
                        missing.join(", ")
                    )));
                }
            }
        }
    }
}

pub(crate) fn analyze_group_expr(stmt: &BoundStatement, expr_id: ExprId) -> ExprAnalysis {
    let mut analysis = ExprAnalysis::default();
    analyze_group_expr_inner(stmt, expr_id, &mut analysis);
    analysis
}

fn analyze_group_expr_inner(stmt: &BoundStatement, expr_id: ExprId, analysis: &mut ExprAnalysis) {
    match stmt.exprs.get(expr_id) {
        BoundExpr::Column(column_id) => {
            analysis.columns.insert(*column_id);
        },
        BoundExpr::Binary { left, right, .. } => {
            analyze_group_expr_inner(stmt, *left, analysis);
            analyze_group_expr_inner(stmt, *right, analysis);
        },
        BoundExpr::Unary { expr, .. } => {
            analyze_group_expr_inner(stmt, *expr, analysis);
        },
        BoundExpr::IsNull { expr, .. } => {
            analyze_group_expr_inner(stmt, *expr, analysis);
        },
        BoundExpr::Function {
            kind, args, over, ..
        } => {
            let is_aggregate_name = matches!(kind, FunctionKind::Aggregate(_));
            let is_window_name = matches!(kind, FunctionKind::Window(_));
            let is_window = is_window_name || (*over && is_aggregate_name);
            let is_aggregate = is_aggregate_name && !is_window;

            if is_window {
                analysis.has_window = true;
                return;
            }

            if is_aggregate {
                analysis.has_aggregate = true;
                return;
            }

            for arg in args {
                analyze_group_expr_inner(stmt, *arg, analysis);
            }
        },
        BoundExpr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(expr_id) = operand {
                analyze_group_expr_inner(stmt, *expr_id, analysis);
            }
            for expr_id in conditions {
                analyze_group_expr_inner(stmt, *expr_id, analysis);
            }
            for expr_id in results {
                analyze_group_expr_inner(stmt, *expr_id, analysis);
            }
            if let Some(expr_id) = else_result {
                analyze_group_expr_inner(stmt, *expr_id, analysis);
            }
        },
        BoundExpr::InList { expr, list, .. } => {
            analyze_group_expr_inner(stmt, *expr, analysis);
            for e in list {
                analyze_group_expr_inner(stmt, *e, analysis);
            }
        },
        BoundExpr::Subquery(_) | BoundExpr::InSubquery { .. } => {},
        BoundExpr::Literal(_) | BoundExpr::Wildcard | BoundExpr::Error => {},
    }
}

fn format_column_ref(stmt: &BoundStatement, column_id: ColumnId) -> String {
    let column = stmt.columns.get(column_id);
    let table = stmt.tables.get(column.table);

    let qualifier = table.alias.clone().or_else(|| match &table.source {
        BoundTableSource::Table { name } | BoundTableSource::Cte { name } => Some(name.clone()),
        BoundTableSource::Derived { .. } => None,
    });

    if let Some(qualifier) = qualifier {
        format!("{qualifier}.{}", column.name)
    } else {
        column.name.clone()
    }
}
