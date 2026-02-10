use sqlparser::ast::{GroupByExpr, Query, Select, SelectItem};

use super::{Algebraizer, validate::analyze_scalar_expr};
use crate::{
    diagnostics::Diagnostic,
    ir::{
        auxiliary::{JoinCondition, ProjectionColumn, SortKey},
        relational::RelationalExpr,
        scalar::ScalarExpr,
    },
    keywords,
};

impl<'a> Algebraizer<'a> {
    pub(super) fn algebraize_query(&mut self, query: &Query) -> Option<RelationalExpr> {
        // Handle CTEs
        self.process_ctes(query.with.as_ref());

        // Build the main body
        let mut expr = self.algebraize_set_expr(&query.body)?;

        // ORDER BY
        if let Some(order_by) = &query.order_by {
            let keys: Vec<SortKey> = order_by
                .exprs
                .iter()
                .map(|ob| SortKey {
                    expr: self.build_scalar_expr(&ob.expr),
                    asc: ob.asc.unwrap_or(true),
                    nulls_first: ob.nulls_first,
                })
                .collect();
            if !keys.is_empty() {
                expr = RelationalExpr::Sort {
                    input: Box::new(expr),
                    keys,
                };
            }
        }

        // LIMIT / OFFSET
        if query.limit.is_some() || query.offset.is_some() {
            let count = query.limit.as_ref().map(|e| self.build_scalar_expr(e));
            let offset = query
                .offset
                .as_ref()
                .map(|o| self.build_scalar_expr(&o.value));
            expr = RelationalExpr::Limit {
                input: Box::new(expr),
                count,
                offset,
            };
        }

        Some(expr)
    }

    /// Convert a SELECT statement into a RelationalExpr tree following SQL's
    /// logical execution order (FROM → WHERE → GROUP BY → HAVING → SELECT → DISTINCT).
    pub(super) fn algebraize_select(&mut self, select: &Select) -> Option<RelationalExpr> {
        // Step 1: FROM → Scan / Join
        let from_expr = self.build_from(&select.from);

        // Build scalar expressions for later phases
        let selection = select
            .selection
            .as_ref()
            .map(|expr| self.build_scalar_expr(expr));

        let group_by = self.build_group_by(&select.group_by);

        let having = select
            .having
            .as_ref()
            .map(|expr| self.build_scalar_expr(expr));

        let projection = self.build_projection(&select.projection);

        // Collect JOIN ON conditions for context validation
        let join_conditions = collect_join_on_conditions(from_expr.as_ref());
        let join_condition_refs: Vec<&ScalarExpr> = join_conditions.iter().collect();

        // Validate contexts (no aggregates in WHERE, no windows in GROUP BY, etc.)
        self.validate_select_contexts(
            selection.as_ref(),
            &group_by,
            having.as_ref(),
            &join_condition_refs,
        );

        // Validate GROUP BY rules
        let projection_scalar_exprs: Vec<ScalarExpr> =
            projection.iter().map(|p| p.expr.clone()).collect();
        self.validate_grouping(&projection_scalar_exprs, &group_by, having.as_ref());

        // Now build the RelationalExpr tree bottom-up
        // If there's no FROM clause (e.g. SELECT 1), use a single-row Values as the base
        let mut expr = from_expr.unwrap_or(RelationalExpr::Values { rows: vec![vec![]] });

        // Step 2: WHERE → Selection
        if let Some(condition) = selection {
            expr = RelationalExpr::Selection {
                input: Box::new(expr),
                condition,
            };
        }

        // Step 3: GROUP BY + aggregates → Aggregation
        let has_aggregate = projection_scalar_exprs
            .iter()
            .any(|e| analyze_scalar_expr(e).has_aggregate)
            || having
                .as_ref()
                .map(|h| analyze_scalar_expr(h).has_aggregate)
                .unwrap_or(false);

        if !group_by.is_empty() || has_aggregate {
            expr = RelationalExpr::Aggregation {
                input: Box::new(expr),
                group_by,
                aggregates: Vec::new(),
            };
        }

        // Step 4: HAVING → Selection
        if let Some(condition) = having {
            expr = RelationalExpr::Selection {
                input: Box::new(expr),
                condition,
            };
        }

        // Step 5: Window functions — handled inline via ScalarExpr::WindowCall

        // Step 6: SELECT → Projection
        expr = RelationalExpr::Projection {
            input: Box::new(expr),
            columns: projection,
        };

        // Step 7: DISTINCT → Distinct
        if select.distinct.is_some() {
            expr = RelationalExpr::Distinct {
                input: Box::new(expr),
            };
        }

        // Steps 8-9 (ORDER BY, LIMIT/OFFSET) are handled in algebraize_query
        Some(expr)
    }

    fn build_group_by(&mut self, group_by: &GroupByExpr) -> Vec<ScalarExpr> {
        match group_by {
            GroupByExpr::Expressions(exprs, _) => {
                exprs.iter().map(|e| self.build_scalar_expr(e)).collect()
            },
            GroupByExpr::All(_) => {
                self.diagnostics
                    .push(Diagnostic::unsupported_feature("GROUP BY ALL"));
                Vec::new()
            },
        }
    }

    fn build_projection(&mut self, items: &[SelectItem]) -> Vec<ProjectionColumn> {
        let mut columns = Vec::new();
        for item in items {
            match item {
                SelectItem::UnnamedExpr(expr) => {
                    columns.push(ProjectionColumn {
                        expr: self.build_scalar_expr(expr),
                        alias: None,
                    });
                },
                SelectItem::ExprWithAlias { expr, alias } => {
                    if keywords::is_reserved_identifier(self.dialect, alias) {
                        self.diagnostics.push(Diagnostic::invalid_statement(format!(
                            "Alias {alias} is a reserved keyword in {}; quote it to use as an identifier",
                            self.dialect
                        )));
                    }
                    columns.push(ProjectionColumn {
                        expr: self.build_scalar_expr(expr),
                        alias: Some(alias.value.clone()),
                    });
                },
                SelectItem::Wildcard(_) => {
                    columns.push(ProjectionColumn {
                        expr: ScalarExpr::Wildcard,
                        alias: None,
                    });
                },
                SelectItem::QualifiedWildcard(name, _) => {
                    let table_name = name.to_string();
                    columns.push(ProjectionColumn {
                        expr: ScalarExpr::QualifiedWildcard { table: table_name },
                        alias: None,
                    });
                },
            }
        }
        columns
    }
}

/// Walk a RelationalExpr tree and collect all JOIN ON scalar expressions
/// for context validation purposes.
fn collect_join_on_conditions(expr: Option<&RelationalExpr>) -> Vec<ScalarExpr> {
    let mut conditions = Vec::new();
    if let Some(expr) = expr {
        collect_join_on_inner(expr, &mut conditions);
    }
    conditions
}

fn collect_join_on_inner(expr: &RelationalExpr, conditions: &mut Vec<ScalarExpr>) {
    match expr {
        RelationalExpr::Join {
            left,
            right,
            condition,
            ..
        } => {
            if let Some(JoinCondition::On(scalar)) = condition {
                conditions.push(scalar.clone());
            }
            collect_join_on_inner(left, conditions);
            collect_join_on_inner(right, conditions);
        },
        RelationalExpr::Alias { input, .. } => {
            collect_join_on_inner(input, conditions);
        },
        _ => {},
    }
}
