use crate::{
    analysis::{
        diagnostics::Diagnostic,
        validate::{Validator, grouping::analyze_group_expr},
    },
    ir::bound::{BoundJoinCondition, BoundSelect, BoundStatement},
};

impl Validator {
    pub(super) fn validate_select_contexts(&mut self, stmt: &BoundStatement, select: &BoundSelect) {
        if let Some(selection) = select.selection {
            let analysis = analyze_group_expr(stmt, selection);
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
                    let analysis = analyze_group_expr(stmt, *expr_id);
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
            let analysis = analyze_group_expr(stmt, *expr_id);
            if analysis.has_window {
                self.diagnostics
                    .push(Diagnostic::group_by_window_not_allowed());
            } else if analysis.has_aggregate {
                self.diagnostics
                    .push(Diagnostic::group_by_aggregate_not_allowed());
            }
        }

        if let Some(having) = select.having {
            let analysis = analyze_group_expr(stmt, having);
            if analysis.has_window {
                self.diagnostics
                    .push(Diagnostic::window_not_allowed("HAVING clause"));
            }
        }
    }
}
