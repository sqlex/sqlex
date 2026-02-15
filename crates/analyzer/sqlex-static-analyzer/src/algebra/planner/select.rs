use sqlparser::ast::{Expr, GroupByExpr, Select, SelectItem};

use crate::{
    algebra::{
        expr::{AggregationNode, ProjectionNode, RelExpr, SelectionNode, WindowNode},
        planner::{Algebraizer, context::BuildContext},
        scalar::{
            BoundColumn, BoundScalarExpr, ColumnOrigin, OutputSchema, ProjectionColumn, Visibility,
        },
    },
    catalog::{model::Catalog, normalize::normalize_object_name},
    diagnostics::{Diagnostic, Phase},
    functions::registry::FunctionRegistry,
};

impl Algebraizer {
    pub(crate) fn build_select(
        &self,
        select: &Select,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &mut BuildContext,
    ) -> Result<RelExpr, Diagnostic> {
        if select.into.is_some()
            || !select.lateral_views.is_empty()
            || select.prewhere.is_some()
            || !select.cluster_by.is_empty()
            || !select.distribute_by.is_empty()
            || !select.sort_by.is_empty()
            || !select.named_window.is_empty()
            || select.qualify.is_some()
            || select.connect_by.is_some()
            || select.top.is_some()
            || select.value_table_mode.is_some()
        {
            return Err(Diagnostic::todo(
                Phase::Algebraize,
                "advanced SELECT clauses",
            ));
        }

        let group_by_exprs = self.group_by_expressions(&select.group_by)?;
        let mut bound_group_by = Vec::with_capacity(group_by_exprs.len());

        context.relation_scopes.clear();
        let mut input_expr = self.build_from(select, catalog, functions, context)?;
        if let Some(selection) = &select.selection {
            let (condition, _) = self.bind_expr(selection, catalog, functions, context)?;
            let schema = super::output_schema_of(&input_expr)?;
            input_expr = RelExpr::Selection(SelectionNode {
                input: Box::new(input_expr),
                condition,
                schema,
            });
        }

        for group_expr in group_by_exprs {
            let (bound_group_expr, has_aggregate) =
                self.bind_expr(group_expr, catalog, functions, context)?;
            if has_aggregate {
                return Err(Diagnostic::new(
                    "A3016",
                    Phase::Algebraize,
                    "aggregate expression is not allowed in GROUP BY",
                ));
            }
            bound_group_by.push(ProjectionColumn {
                expr: bound_group_expr,
                alias: None,
                visibility: Visibility::Visible,
            });
        }

        let group_by_count = bound_group_by.len();
        let mut has_aggregate = false;
        let mut has_window = false;
        let mut aggregate_exprs = Vec::new();
        let mut window_exprs = Vec::new();
        if let Some(having_expr) = &select.having {
            let (condition, having_has_aggregate) =
                self.bind_expr(having_expr, catalog, functions, context)?;
            has_aggregate |= having_has_aggregate;
            if having_has_aggregate {
                aggregate_exprs.push(ProjectionColumn {
                    expr: condition.clone(),
                    alias: None,
                    visibility: Visibility::Hidden,
                });
            }
            if contains_window_call(&condition) {
                has_window = true;
                window_exprs.push(ProjectionColumn {
                    expr: condition.clone(),
                    alias: Some("__window$having".to_string()),
                    visibility: Visibility::Hidden,
                });
            }

            let schema = super::output_schema_of(&input_expr)?;
            input_expr = RelExpr::Selection(SelectionNode {
                input: Box::new(input_expr),
                condition,
                schema,
            });
        }

        let input_schema = super::output_schema_of(&input_expr)?;
        let mut projected_columns = Vec::new();
        let mut projection_schema_columns = Vec::new();
        let mut has_non_aggregate_projection = false;

        for select_item in &select.projection {
            match select_item {
                SelectItem::Wildcard(_) => {
                    has_non_aggregate_projection = true;
                    for column in &input_schema.columns {
                        projected_columns.push(ProjectionColumn {
                            expr: BoundScalarExpr::SlotRef(column.slot_id),
                            alias: Some(column.name.clone()),
                            visibility: Visibility::Visible,
                        });
                        projection_schema_columns.push(BoundColumn {
                            slot_id: context.allocate_slot_id(),
                            name: column.name.clone(),
                            table_alias: None,
                            data_type: None,
                            nullable: true,
                            origin: column.origin.clone(),
                        });
                    }
                },
                SelectItem::QualifiedWildcard(qualifier, _) => {
                    has_non_aggregate_projection = true;
                    let qualifier_name = normalize_object_name(qualifier, self.dialect);
                    let Some(scope) = context.relation_scopes.iter().find(|scope| {
                        scope
                            .visible_names
                            .iter()
                            .any(|name| name == &qualifier_name)
                    }) else {
                        return Err(Diagnostic::new(
                            "A3002",
                            Phase::Algebraize,
                            format!("unknown qualified wildcard target: {qualifier_name}"),
                        ));
                    };
                    let scoped_columns = scope.schema.columns.clone();

                    for column in &scoped_columns {
                        projected_columns.push(ProjectionColumn {
                            expr: BoundScalarExpr::SlotRef(column.slot_id),
                            alias: Some(column.name.clone()),
                            visibility: Visibility::Visible,
                        });
                        projection_schema_columns.push(BoundColumn {
                            slot_id: context.allocate_slot_id(),
                            name: column.name.clone(),
                            table_alias: None,
                            data_type: None,
                            nullable: true,
                            origin: column.origin.clone(),
                        });
                    }
                },
                SelectItem::ExprWithAlias { expr, alias } => {
                    self.validate_alias_ident(alias)?;
                    let (bound_expr, expr_has_aggregate) =
                        self.bind_expr(expr, catalog, functions, context)?;
                    let expr_has_window = contains_window_call(&bound_expr);
                    has_aggregate |= expr_has_aggregate;
                    has_window |= expr_has_window;
                    if !expr_has_aggregate {
                        has_non_aggregate_projection = true;
                    } else {
                        aggregate_exprs.push(ProjectionColumn {
                            expr: bound_expr.clone(),
                            alias: None,
                            visibility: Visibility::Hidden,
                        });
                    }
                    if expr_has_window {
                        window_exprs.push(ProjectionColumn {
                            expr: bound_expr.clone(),
                            alias: Some("__window$expr_alias".to_string()),
                            visibility: Visibility::Hidden,
                        });
                    }

                    let output_name = alias.value.clone();
                    projected_columns.push(ProjectionColumn {
                        expr: bound_expr,
                        alias: Some(output_name.clone()),
                        visibility: Visibility::Visible,
                    });
                    projection_schema_columns.push(BoundColumn {
                        slot_id: context.allocate_slot_id(),
                        name: output_name,
                        table_alias: None,
                        data_type: None,
                        nullable: true,
                        origin: ColumnOrigin::Derived,
                    });
                },
                SelectItem::UnnamedExpr(expr) => {
                    let (bound_expr, expr_has_aggregate) =
                        self.bind_expr(expr, catalog, functions, context)?;
                    let expr_has_window = contains_window_call(&bound_expr);
                    has_aggregate |= expr_has_aggregate;
                    has_window |= expr_has_window;
                    if !expr_has_aggregate {
                        has_non_aggregate_projection = true;
                    } else {
                        aggregate_exprs.push(ProjectionColumn {
                            expr: bound_expr.clone(),
                            alias: None,
                            visibility: Visibility::Hidden,
                        });
                    }
                    if expr_has_window {
                        window_exprs.push(ProjectionColumn {
                            expr: bound_expr.clone(),
                            alias: Some("__window$unnamed".to_string()),
                            visibility: Visibility::Hidden,
                        });
                    }

                    let output_name = self.derive_output_name(expr)?;
                    projected_columns.push(ProjectionColumn {
                        expr: bound_expr,
                        alias: Some(output_name.clone()),
                        visibility: Visibility::Visible,
                    });
                    projection_schema_columns.push(BoundColumn {
                        slot_id: context.allocate_slot_id(),
                        name: output_name,
                        table_alias: None,
                        data_type: None,
                        nullable: true,
                        origin: ColumnOrigin::Derived,
                    });
                },
            }
        }

        if group_by_count == 0 && has_aggregate && has_non_aggregate_projection {
            return Err(Diagnostic::new(
                "A3017",
                Phase::Algebraize,
                "non-aggregated projection is not allowed when GROUP BY is absent",
            ));
        }

        let mut relational_expr = input_expr;
        if has_aggregate || group_by_count > 0 {
            let schema = super::output_schema_of(&relational_expr)?;
            relational_expr = RelExpr::Aggregation(AggregationNode {
                input: Box::new(relational_expr),
                group_by: bound_group_by,
                aggregates: aggregate_exprs,
                schema,
            });
        }

        if has_window {
            let schema = super::output_schema_of(&relational_expr)?;
            relational_expr = RelExpr::Window(WindowNode {
                input: Box::new(relational_expr),
                window_exprs,
                schema,
            });
        }

        relational_expr = RelExpr::Projection(ProjectionNode {
            input: Box::new(relational_expr),
            columns: projected_columns,
            schema: OutputSchema {
                relation_id: context.allocate_relation_id(),
                columns: projection_schema_columns,
            },
            is_aggregate: false,
            group_by_count,
        });

        if select.distinct.is_some() {
            let schema = super::output_schema_of(&relational_expr)?;
            relational_expr = RelExpr::Distinct(crate::algebra::expr::DistinctNode {
                input: Box::new(relational_expr),
                schema,
            });
        }

        Ok(relational_expr)
    }

    fn group_by_expressions<'a>(
        &self,
        group_by: &'a GroupByExpr,
    ) -> Result<&'a [Expr], Diagnostic> {
        match group_by {
            GroupByExpr::Expressions(expressions, _) => Ok(expressions.as_slice()),
            _ => Err(Diagnostic::todo(
                Phase::Algebraize,
                "GROUP BY ALL planning and binding",
            )),
        }
    }
}

fn contains_window_call(expr: &BoundScalarExpr) -> bool {
    match expr {
        BoundScalarExpr::WindowCall { .. } => true,
        BoundScalarExpr::BinaryOp { left, right, .. } => {
            contains_window_call(left) || contains_window_call(right)
        },
        BoundScalarExpr::UnaryOp { expr, .. } => contains_window_call(expr),
        BoundScalarExpr::Function { args, .. } | BoundScalarExpr::AggregateCall { args, .. } => {
            args.iter().any(contains_window_call)
        },
        BoundScalarExpr::Cast { expr, .. }
        | BoundScalarExpr::IsNull { expr, .. }
        | BoundScalarExpr::InSubquery { expr, .. } => contains_window_call(expr),
        BoundScalarExpr::Case {
            operand,
            when_clauses,
            else_expr,
        } => {
            operand
                .as_ref()
                .is_some_and(|value| contains_window_call(value))
                || when_clauses.iter().any(|(condition, result)| {
                    contains_window_call(condition) || contains_window_call(result)
                })
                || else_expr
                    .as_ref()
                    .is_some_and(|value| contains_window_call(value))
        },
        BoundScalarExpr::InList { expr, list, .. } => {
            contains_window_call(expr) || list.iter().any(contains_window_call)
        },
        BoundScalarExpr::SlotRef(_)
        | BoundScalarExpr::CorrelatedRef { .. }
        | BoundScalarExpr::Literal(_)
        | BoundScalarExpr::Exists { .. }
        | BoundScalarExpr::ScalarSubquery { .. }
        | BoundScalarExpr::Placeholder(_) => false,
    }
}
