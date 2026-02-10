use super::Algebraizer;
use crate::{diagnostics::Diagnostic, ir::scalar::ScalarExpr};

/// Analysis result for a scalar expression in the context of GROUP BY validation.
#[derive(Default)]
pub(super) struct ExprAnalysis {
    /// Column references found in the expression (table.column or just column)
    pub(super) column_refs: Vec<(Option<String>, String)>,
    pub(super) has_aggregate: bool,
    pub(super) has_window: bool,
}

impl ExprAnalysis {
    pub(super) fn has_grouping_function(&self) -> bool {
        self.has_aggregate || self.has_window
    }
}

impl<'a> Algebraizer<'a> {
    /// Validate that aggregates and window functions do not appear in forbidden contexts.
    pub(super) fn validate_select_contexts(
        &mut self,
        selection: Option<&ScalarExpr>,
        group_by: &[ScalarExpr],
        having: Option<&ScalarExpr>,
        join_conditions: &[&ScalarExpr],
    ) {
        // WHERE clause: no aggregates, no window functions
        if let Some(where_expr) = selection {
            let analysis = analyze_scalar_expr(where_expr);
            if analysis.has_aggregate {
                self.diagnostics
                    .push(Diagnostic::aggregate_not_allowed("WHERE clause"));
            }
            if analysis.has_window {
                self.diagnostics
                    .push(Diagnostic::window_not_allowed("WHERE clause"));
            }
        }

        // JOIN ON conditions: no aggregates, no window functions
        for condition in join_conditions {
            let analysis = analyze_scalar_expr(condition);
            if analysis.has_aggregate {
                self.diagnostics
                    .push(Diagnostic::aggregate_not_allowed("JOIN ON clause"));
            }
            if analysis.has_window {
                self.diagnostics
                    .push(Diagnostic::window_not_allowed("JOIN ON clause"));
            }
        }

        // GROUP BY: no window functions, no aggregates
        for expr in group_by {
            let analysis = analyze_scalar_expr(expr);
            if analysis.has_window {
                self.diagnostics
                    .push(Diagnostic::group_by_window_not_allowed());
            } else if analysis.has_aggregate {
                self.diagnostics
                    .push(Diagnostic::group_by_aggregate_not_allowed());
            }
        }

        // HAVING: no window functions
        if let Some(having_expr) = having {
            let analysis = analyze_scalar_expr(having_expr);
            if analysis.has_window {
                self.diagnostics
                    .push(Diagnostic::window_not_allowed("HAVING clause"));
            }
        }
    }

    /// Validate GROUP BY rules: non-aggregated columns in SELECT/HAVING must appear in GROUP BY.
    pub(super) fn validate_grouping(
        &mut self,
        projection_exprs: &[ScalarExpr],
        group_by: &[ScalarExpr],
        having: Option<&ScalarExpr>,
    ) {
        let has_group_by = !group_by.is_empty();

        // Collect column refs that appear in GROUP BY expressions
        let mut group_columns: Vec<(Option<String>, String)> = Vec::new();
        for expr in group_by {
            let analysis = analyze_scalar_expr(expr);
            group_columns.extend(analysis.column_refs);
        }

        // Check if any projection or HAVING expression uses aggregates
        let mut has_aggregate = false;
        for expr in projection_exprs {
            if analyze_scalar_expr(expr).has_aggregate {
                has_aggregate = true;
                break;
            }
        }
        if !has_aggregate {
            if let Some(h) = having {
                if analyze_scalar_expr(h).has_aggregate {
                    has_aggregate = true;
                }
            }
        }

        let require_grouping = has_group_by || has_aggregate;
        if !require_grouping {
            return;
        }

        // SQLite is lenient about GROUP BY rules
        if self.dialect == sqlex_common::dialect::Dialect::SQLite {
            return;
        }

        // Validate projection columns
        for expr in projection_exprs {
            let analysis = analyze_scalar_expr(expr);
            if analysis.has_grouping_function() {
                continue;
            }
            let missing: Vec<String> = analysis
                .column_refs
                .iter()
                .filter(|col| !column_in_group(&group_columns, col))
                .map(format_column_ref)
                .collect();
            if !missing.is_empty() {
                self.diagnostics.push(Diagnostic::grouping_error(format!(
                    "Non-aggregated SELECT columns must appear in GROUP BY: {}",
                    missing.join(", ")
                )));
            }
        }

        // Validate HAVING
        if let Some(having_expr) = having {
            let analysis = analyze_scalar_expr(having_expr);
            if !analysis.has_grouping_function() {
                let missing: Vec<String> = analysis
                    .column_refs
                    .iter()
                    .filter(|col| !column_in_group(&group_columns, col))
                    .map(format_column_ref)
                    .collect();
                if !missing.is_empty() {
                    self.diagnostics.push(Diagnostic::grouping_error(format!(
                        "HAVING references non-grouped columns: {}",
                        missing.join(", ")
                    )));
                }
            }
        }
    }
}

pub(super) fn analyze_scalar_expr(expr: &ScalarExpr) -> ExprAnalysis {
    let mut analysis = ExprAnalysis::default();
    analyze_scalar_expr_inner(expr, &mut analysis);
    analysis
}

fn analyze_scalar_expr_inner(expr: &ScalarExpr, analysis: &mut ExprAnalysis) {
    match expr {
        ScalarExpr::ColumnRef { table, column } => {
            analysis.column_refs.push((table.clone(), column.clone()));
        },
        ScalarExpr::BinaryOp { left, right, .. } => {
            analyze_scalar_expr_inner(left, analysis);
            analyze_scalar_expr_inner(right, analysis);
        },
        ScalarExpr::UnaryOp { expr, .. } => {
            analyze_scalar_expr_inner(expr, analysis);
        },
        ScalarExpr::IsNull { expr, .. } => {
            analyze_scalar_expr_inner(expr, analysis);
        },
        ScalarExpr::Function { args, .. } => {
            for arg in args {
                analyze_scalar_expr_inner(arg, analysis);
            }
        },
        ScalarExpr::AggregateCall { .. } => {
            analysis.has_aggregate = true;
        },
        ScalarExpr::WindowCall { .. } => {
            analysis.has_window = true;
        },
        ScalarExpr::Case {
            operand,
            when_clauses,
            else_result,
        } => {
            if let Some(op) = operand {
                analyze_scalar_expr_inner(op, analysis);
            }
            for clause in when_clauses {
                analyze_scalar_expr_inner(&clause.condition, analysis);
                analyze_scalar_expr_inner(&clause.result, analysis);
            }
            if let Some(el) = else_result {
                analyze_scalar_expr_inner(el, analysis);
            }
        },
        ScalarExpr::InList { expr, list, .. } => {
            analyze_scalar_expr_inner(expr, analysis);
            for e in list {
                analyze_scalar_expr_inner(e, analysis);
            }
        },
        ScalarExpr::Between {
            expr, low, high, ..
        } => {
            analyze_scalar_expr_inner(expr, analysis);
            analyze_scalar_expr_inner(low, analysis);
            analyze_scalar_expr_inner(high, analysis);
        },
        ScalarExpr::Cast { expr, .. } => {
            analyze_scalar_expr_inner(expr, analysis);
        },
        ScalarExpr::ScalarSubquery(_)
        | ScalarExpr::InSubquery { .. }
        | ScalarExpr::Exists { .. }
        | ScalarExpr::Literal(_)
        | ScalarExpr::Wildcard
        | ScalarExpr::QualifiedWildcard { .. }
        | ScalarExpr::Error => {},
    }
}

fn column_in_group(
    group_columns: &[(Option<String>, String)],
    col: &(Option<String>, String),
) -> bool {
    group_columns
        .iter()
        .any(|gc| gc.1 == col.1 && (gc.0 == col.0 || gc.0.is_none() || col.0.is_none()))
}

fn format_column_ref(col: &(Option<String>, String)) -> String {
    match &col.0 {
        Some(table) => format!("{table}.{}", col.1),
        None => col.1.clone(),
    }
}
