use std::collections::HashSet;

use crate::{
    analysis::{
        diagnostics::Diagnostic,
        functions::{FunctionKind, resolve_function},
        typecheck::{QueryTypeState, TypeContext},
    },
    ir::{
        bound::{BoundExpr, BoundJoinCondition, BoundSelect, BoundTableSource},
        ids::{ColumnId, ExprId},
    },
};

impl<'a> TypeContext<'a> {
    pub(super) fn validate_grouping(&mut self, state: &QueryTypeState<'_>, select: &BoundSelect) {
        let has_group_by = !select.group_by.is_empty();

        let mut group_columns = HashSet::new();
        for expr_id in &select.group_by {
            let analysis = self.analyze_group_expr(state, *expr_id);
            group_columns.extend(analysis.columns);
        }

        let mut has_aggregate = false;
        for proj in &select.projection {
            if self.analyze_group_expr(state, proj.expr).has_aggregate {
                has_aggregate = true;
            }
        }
        if let Some(having) = select.having {
            if self.analyze_group_expr(state, having).has_aggregate {
                has_aggregate = true;
            }
        }

        let require_grouping = has_group_by || has_aggregate;
        if !require_grouping {
            return;
        }

        if self.dialect != sqlex_common::dialect::Dialect::SQLite {
            for proj in &select.projection {
                let analysis = self.analyze_group_expr(state, proj.expr);
                if analysis.has_grouping_function() {
                    continue;
                }
                if !analysis.columns.is_subset(&group_columns) {
                    let missing = analysis
                        .columns
                        .difference(&group_columns)
                        .map(|col| self.format_column_ref(state, *col))
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
                let analysis = self.analyze_group_expr(state, having);
                if !analysis.has_grouping_function() && !analysis.columns.is_subset(&group_columns)
                {
                    let missing = analysis
                        .columns
                        .difference(&group_columns)
                        .map(|col| self.format_column_ref(state, *col))
                        .collect::<Vec<_>>();
                    self.diagnostics.push(Diagnostic::grouping_error(format!(
                        "HAVING references non-grouped columns: {}",
                        missing.join(", ")
                    )));
                }
            }
        }
    }

    pub(super) fn validate_select_contexts(
        &mut self,
        state: &QueryTypeState<'_>,
        select: &BoundSelect,
    ) {
        if let Some(selection) = select.selection {
            let analysis = self.analyze_group_expr(state, selection);
            if analysis.has_aggregate {
                self.diagnostics
                    .push(Diagnostic::aggregate_not_allowed("WHERE clause"));
            }
            if analysis.has_window {
                self.diagnostics
                    .push(Diagnostic::window_not_allowed("WHERE clause"));
            }
        }

        for from_item in &select.from {
            for join in &from_item.joins {
                if let Some(BoundJoinCondition::On(expr_id)) = &join.condition {
                    let analysis = self.analyze_group_expr(state, *expr_id);
                    if analysis.has_aggregate {
                        self.diagnostics
                            .push(Diagnostic::aggregate_not_allowed("JOIN ON clause"));
                    }
                    if analysis.has_window {
                        self.diagnostics
                            .push(Diagnostic::window_not_allowed("JOIN ON clause"));
                    }
                }
            }
        }

        for expr_id in &select.group_by {
            let analysis = self.analyze_group_expr(state, *expr_id);
            if analysis.has_window {
                self.diagnostics
                    .push(Diagnostic::group_by_window_not_allowed());
            } else if analysis.has_aggregate {
                self.diagnostics
                    .push(Diagnostic::group_by_aggregate_not_allowed());
            }
        }

        if let Some(having) = select.having {
            let analysis = self.analyze_group_expr(state, having);
            if analysis.has_window {
                self.diagnostics
                    .push(Diagnostic::window_not_allowed("HAVING clause"));
            }
        }
    }

    pub(super) fn analyze_group_expr(
        &self,
        state: &QueryTypeState<'_>,
        expr_id: ExprId,
    ) -> ExprAnalysis {
        let mut analysis = ExprAnalysis::default();
        self.analyze_group_expr_inner(state, expr_id, &mut analysis);
        analysis
    }

    fn analyze_group_expr_inner(
        &self,
        state: &QueryTypeState<'_>,
        expr_id: ExprId,
        analysis: &mut ExprAnalysis,
    ) {
        match state.query.exprs.get(expr_id) {
            BoundExpr::Column(column_id) => {
                analysis.columns.insert(*column_id);
            },
            BoundExpr::Binary { left, right, .. } => {
                self.analyze_group_expr_inner(state, *left, analysis);
                self.analyze_group_expr_inner(state, *right, analysis);
            },
            BoundExpr::Unary { expr, .. } => {
                self.analyze_group_expr_inner(state, *expr, analysis);
            },
            BoundExpr::IsNull { expr, .. } => {
                self.analyze_group_expr_inner(state, *expr, analysis);
            },
            BoundExpr::Function {
                name, args, over, ..
            } => {
                let meta = resolve_function(name);
                let is_aggregate_name = matches!(meta.kind, FunctionKind::Aggregate(_));
                let is_window_name = matches!(meta.kind, FunctionKind::Window(_));
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
                    self.analyze_group_expr_inner(state, *arg, analysis);
                }
            },
            BoundExpr::Case {
                operand,
                conditions,
                results,
                else_result,
            } => {
                if let Some(expr_id) = operand {
                    self.analyze_group_expr_inner(state, *expr_id, analysis);
                }
                for expr_id in conditions {
                    self.analyze_group_expr_inner(state, *expr_id, analysis);
                }
                for expr_id in results {
                    self.analyze_group_expr_inner(state, *expr_id, analysis);
                }
                if let Some(expr_id) = else_result {
                    self.analyze_group_expr_inner(state, *expr_id, analysis);
                }
            },
            BoundExpr::Subquery(_) => {
                // Scalar subqueries should not force grouping in the outer query.
            },
            _ => {},
        }
    }

    fn format_column_ref(&self, state: &QueryTypeState<'_>, column_id: ColumnId) -> String {
        let column = state.query.columns.get(column_id);
        let table = state.query.tables.get(column.table);

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
}

#[derive(Default)]
pub(super) struct ExprAnalysis {
    columns: HashSet<crate::ir::ids::ColumnId>,
    pub(super) has_aggregate: bool,
    has_window: bool,
}

impl ExprAnalysis {
    fn has_grouping_function(&self) -> bool {
        self.has_aggregate || self.has_window
    }
}
