use std::collections::{HashMap, HashSet};

use sqlparser::ast::{
    Expr, GroupByExpr, NamedWindowDefinition, NamedWindowExpr, Select, SelectItem, WindowSpec,
};

use crate::{
    algebraizer::{
        Algebraizer,
        context::BuildContext,
        model::{
            expression::Expression,
            relation::{AggregationNode, ProjectionNode, Relation, SelectionNode, WindowNode},
            schema::{BoundColumn, ColumnOrigin, OutputSchema, ProjectionColumn, Visibility},
        },
    },
    catalog::{
        Catalog,
        normalize::{normalize_ident, normalize_object_name},
    },
    diagnostics::{Diagnostic, Phase},
    functions::FunctionRegistry,
};

impl Algebraizer {
    pub(crate) fn build_select(
        &self,
        select: &Select,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &mut BuildContext,
    ) -> Result<Relation, Diagnostic> {
        if select.into.is_some()
            || !select.lateral_views.is_empty()
            || select.prewhere.is_some()
            || !select.cluster_by.is_empty()
            || !select.distribute_by.is_empty()
            || !select.sort_by.is_empty()
            || select.qualify.is_some()
            || select.connect_by.is_some()
            || select.top.is_some()
            || select.value_table_mode.is_some()
        {
            return Err(Diagnostic::new(
                "A3051",
                Phase::Algebraize,
                "advanced SELECT clauses are not supported in this planner path",
            ));
        }

        let previous_scope_level = context.relation_scopes.clone();
        let previous_named_windows = context.named_windows.clone();
        let has_outer_scope = !previous_scope_level.is_empty();
        if has_outer_scope {
            context
                .outer_relation_scopes
                .push(previous_scope_level.clone());
        }
        context.named_windows.clear();

        let build_result = (|| {
            self.register_named_windows(&select.named_window, context)?;

            let group_by_exprs = self.group_by_expressions(&select.group_by)?;
            let mut bound_group_by = Vec::with_capacity(group_by_exprs.len());

            context.relation_scopes.clear();
            let mut input_expr = self.build_from(select, catalog, functions, context)?;
            if let Some(selection) = &select.selection {
                let (condition, where_has_aggregate) =
                    self.bind_expression(selection, catalog, functions, context)?;
                if where_has_aggregate {
                    return Err(Diagnostic::new(
                        "A3041",
                        Phase::Algebraize,
                        "aggregate expression is not allowed in WHERE",
                    ));
                }
                if contains_window_call(&condition) {
                    return Err(Diagnostic::new(
                        "A3042",
                        Phase::Algebraize,
                        "window expression is not allowed in WHERE",
                    ));
                }
                let schema = super::output_schema_of(&input_expr)?;
                input_expr = Relation::Selection(SelectionNode {
                    input: Box::new(input_expr),
                    condition,
                    schema,
                });
            }

            for group_expr in group_by_exprs {
                let (bound_group_expr, has_aggregate) =
                    self.bind_expression(group_expr, catalog, functions, context)?;
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
            let mut bound_having = None;
            if let Some(having_expr) = &select.having {
                let (condition, having_has_aggregate) =
                    self.bind_expression(having_expr, catalog, functions, context)?;
                if contains_window_call(&condition) {
                    return Err(Diagnostic::new(
                        "A3043",
                        Phase::Algebraize,
                        "window expression is not allowed in HAVING",
                    ));
                }
                has_aggregate |= having_has_aggregate;
                if having_has_aggregate {
                    aggregate_exprs.push(ProjectionColumn {
                        expr: condition.clone(),
                        alias: None,
                        visibility: Visibility::Hidden,
                    });
                }
                bound_having = Some((condition, having_has_aggregate));
            }

            let input_schema = super::output_schema_of(&input_expr)?;
            let mut projected_columns = Vec::new();
            let mut projection_schema_columns = Vec::new();
            let mut has_non_aggregate_projection = false;
            let mut projection_checks = Vec::new();

            for select_item in &select.projection {
                match select_item {
                    SelectItem::Wildcard(_) => {
                        has_non_aggregate_projection = true;
                        for column in &input_schema.columns {
                            let bound_expr = Expression::SlotRef(column.slot_id);
                            projected_columns.push(ProjectionColumn {
                                expr: bound_expr.clone(),
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
                            projection_checks.push((column.name.clone(), bound_expr, false));
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
                            let bound_expr = Expression::SlotRef(column.slot_id);
                            projected_columns.push(ProjectionColumn {
                                expr: bound_expr.clone(),
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
                            projection_checks.push((column.name.clone(), bound_expr, false));
                        }
                    },
                    SelectItem::ExprWithAlias { expr, alias } => {
                        self.validate_alias_ident(alias)?;
                        let (bound_expr, expr_has_aggregate) =
                            self.bind_expression(expr, catalog, functions, context)?;
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
                            expr: bound_expr.clone(),
                            alias: Some(output_name.clone()),
                            visibility: Visibility::Visible,
                        });
                        projection_schema_columns.push(BoundColumn {
                            slot_id: context.allocate_slot_id(),
                            name: output_name.clone(),
                            table_alias: None,
                            data_type: None,
                            nullable: true,
                            origin: ColumnOrigin::Derived,
                        });
                        projection_checks.push((output_name, bound_expr, expr_has_aggregate));
                    },
                    SelectItem::UnnamedExpr(expr) => {
                        let (bound_expr, expr_has_aggregate) =
                            self.bind_expression(expr, catalog, functions, context)?;
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
                            expr: bound_expr.clone(),
                            alias: Some(output_name.clone()),
                            visibility: Visibility::Visible,
                        });
                        projection_schema_columns.push(BoundColumn {
                            slot_id: context.allocate_slot_id(),
                            name: output_name.clone(),
                            table_alias: None,
                            data_type: None,
                            nullable: true,
                            origin: ColumnOrigin::Derived,
                        });
                        projection_checks.push((output_name, bound_expr, expr_has_aggregate));
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

            if has_aggregate || group_by_count > 0 {
                let grouped_slots = grouped_slot_ids(&bound_group_by);
                for (name, expr, _) in &projection_checks {
                    if contains_ungrouped_slot_outside_aggregate(expr, &grouped_slots, false) {
                        return Err(Diagnostic::new(
                            "A3044",
                            Phase::Algebraize,
                            format!(
                                "projection expression '{name}' must reference grouped columns or aggregates",
                            ),
                        ));
                    }
                }
                if let Some((having_condition, _)) = &bound_having {
                    if contains_ungrouped_slot_outside_aggregate(
                        having_condition,
                        &grouped_slots,
                        false,
                    ) {
                        return Err(Diagnostic::new(
                            "A3045",
                            Phase::Algebraize,
                            "HAVING expression must reference grouped columns or aggregates",
                        ));
                    }
                }
            }

            let mut relational_expr = input_expr;
            if has_aggregate || group_by_count > 0 {
                let schema = super::output_schema_of(&relational_expr)?;
                relational_expr = Relation::Aggregation(AggregationNode {
                    input: Box::new(relational_expr),
                    group_by: bound_group_by,
                    aggregates: aggregate_exprs,
                    schema,
                });
            }

            if let Some((condition, _)) = bound_having {
                let schema = super::output_schema_of(&relational_expr)?;
                relational_expr = Relation::Selection(SelectionNode {
                    input: Box::new(relational_expr),
                    condition,
                    schema,
                });
            }

            if has_window {
                let schema = super::output_schema_of(&relational_expr)?;
                relational_expr = Relation::Window(WindowNode {
                    input: Box::new(relational_expr),
                    window_exprs,
                    schema,
                });
            }

            relational_expr = Relation::Projection(ProjectionNode {
                input: Box::new(relational_expr),
                columns: projected_columns,
                schema: OutputSchema {
                    relation_id: context.allocate_relation_id(),
                    columns: projection_schema_columns,
                },
            });

            if select.distinct.is_some() {
                let schema = super::output_schema_of(&relational_expr)?;
                relational_expr =
                    Relation::Distinct(crate::algebraizer::model::relation::DistinctNode {
                        input: Box::new(relational_expr),
                        schema,
                    });
            }

            Ok(relational_expr)
        })();

        context.relation_scopes = previous_scope_level;
        context.named_windows = previous_named_windows;
        if has_outer_scope {
            let _ = context.outer_relation_scopes.pop();
        }

        build_result
    }

    fn group_by_expressions<'a>(
        &self,
        group_by: &'a GroupByExpr,
    ) -> Result<&'a [Expr], Diagnostic> {
        match group_by {
            GroupByExpr::Expressions(expressions, _) => Ok(expressions.as_slice()),
            _ => Err(Diagnostic::new(
                "A3052",
                Phase::Algebraize,
                "GROUP BY form is not supported in this planner path",
            )),
        }
    }

    fn register_named_windows(
        &self,
        definitions: &[NamedWindowDefinition],
        context: &mut BuildContext,
    ) -> Result<(), Diagnostic> {
        if definitions.is_empty() {
            return Ok(());
        }

        let mut raw_definitions = HashMap::new();
        for NamedWindowDefinition(name, expr) in definitions {
            let normalized_name = normalize_ident(name, self.dialect);
            if raw_definitions
                .insert(normalized_name.clone(), expr.clone())
                .is_some()
            {
                return Err(Diagnostic::new(
                    "A3046",
                    Phase::Algebraize,
                    format!("duplicate WINDOW definition: {normalized_name}"),
                ));
            }
        }

        let mut resolved = HashMap::new();
        for name in raw_definitions.keys() {
            let mut resolving_stack = Vec::new();
            let spec = self.resolve_named_window_spec(
                name,
                &raw_definitions,
                &mut resolved,
                &mut resolving_stack,
            )?;
            resolved.insert(name.clone(), spec);
        }

        context.named_windows = resolved;
        Ok(())
    }

    fn resolve_named_window_spec(
        &self,
        name: &str,
        definitions: &HashMap<String, NamedWindowExpr>,
        resolved: &mut HashMap<String, WindowSpec>,
        resolving_stack: &mut Vec<String>,
    ) -> Result<WindowSpec, Diagnostic> {
        if let Some(spec) = resolved.get(name) {
            return Ok(spec.clone());
        }

        if resolving_stack.iter().any(|item| item == name) {
            return Err(Diagnostic::new(
                "A3047",
                Phase::Algebraize,
                format!("cyclic WINDOW definition: {name}"),
            ));
        }

        let Some(definition) = definitions.get(name) else {
            return Err(Diagnostic::new(
                "A3048",
                Phase::Algebraize,
                format!("unknown WINDOW definition: {name}"),
            ));
        };

        resolving_stack.push(name.to_string());
        let resolved_spec = match definition {
            NamedWindowExpr::NamedWindow(base_name) => {
                let normalized_base = normalize_ident(base_name, self.dialect);
                self.resolve_named_window_spec(
                    &normalized_base,
                    definitions,
                    resolved,
                    resolving_stack,
                )?
            },
            NamedWindowExpr::WindowSpec(spec) => {
                let mut merged = if let Some(base_name) = &spec.window_name {
                    let normalized_base = normalize_ident(base_name, self.dialect);
                    self.resolve_named_window_spec(
                        &normalized_base,
                        definitions,
                        resolved,
                        resolving_stack,
                    )?
                } else {
                    WindowSpec {
                        window_name: None,
                        partition_by: Vec::new(),
                        order_by: Vec::new(),
                        window_frame: None,
                    }
                };

                if !spec.partition_by.is_empty() {
                    merged.partition_by = spec.partition_by.clone();
                }
                if !spec.order_by.is_empty() {
                    merged.order_by = spec.order_by.clone();
                }
                if spec.window_frame.is_some() {
                    merged.window_frame = spec.window_frame.clone();
                }
                merged.window_name = None;
                merged
            },
        };
        let _ = resolving_stack.pop();

        resolved.insert(name.to_string(), resolved_spec.clone());
        Ok(resolved_spec)
    }
}

fn contains_window_call(expr: &Expression) -> bool {
    match expr {
        Expression::WindowCall { .. } => true,
        Expression::BinaryOp { left, right, .. } => {
            contains_window_call(left) || contains_window_call(right)
        },
        Expression::UnaryOp { expr, .. } => contains_window_call(expr),
        Expression::Function { args, .. } | Expression::AggregateCall { args, .. } => {
            args.iter().any(contains_window_call)
        },
        Expression::Cast { expr, .. }
        | Expression::IsNull { expr, .. }
        | Expression::InSubquery { expr, .. } => contains_window_call(expr),
        Expression::Case {
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
        Expression::InList { expr, list, .. } => {
            contains_window_call(expr) || list.iter().any(contains_window_call)
        },
        Expression::SlotRef(_)
        | Expression::CorrelatedRef { .. }
        | Expression::Literal(_)
        | Expression::Exists { .. }
        | Expression::ScalarSubquery(_)
        | Expression::Placeholder => false,
    }
}

fn grouped_slot_ids(group_by: &[ProjectionColumn]) -> HashSet<u32> {
    let mut grouped_slots = HashSet::new();
    for projection in group_by {
        collect_slot_refs(&projection.expr, &mut grouped_slots);
    }
    grouped_slots
}

fn collect_slot_refs(expr: &Expression, slot_ids: &mut HashSet<u32>) {
    match expr {
        Expression::SlotRef(slot_id) => {
            let _ = slot_ids.insert(*slot_id);
        },
        Expression::BinaryOp { left, right, .. } => {
            collect_slot_refs(left, slot_ids);
            collect_slot_refs(right, slot_ids);
        },
        Expression::UnaryOp { expr, .. }
        | Expression::Cast { expr, .. }
        | Expression::IsNull { expr, .. }
        | Expression::InSubquery { expr, .. } => collect_slot_refs(expr, slot_ids),
        Expression::Function { args, .. } | Expression::AggregateCall { args, .. } => {
            for arg in args {
                collect_slot_refs(arg, slot_ids);
            }
        },
        Expression::WindowCall {
            args,
            order_by,
            partition_by,
            ..
        } => {
            for arg in args {
                collect_slot_refs(arg, slot_ids);
            }
            for expr in partition_by {
                collect_slot_refs(expr, slot_ids);
            }
            for key in order_by {
                collect_slot_refs(&key.expr, slot_ids);
            }
        },
        Expression::Case {
            operand,
            when_clauses,
            else_expr,
        } => {
            if let Some(operand) = operand {
                collect_slot_refs(operand, slot_ids);
            }
            for (condition, result) in when_clauses {
                collect_slot_refs(condition, slot_ids);
                collect_slot_refs(result, slot_ids);
            }
            if let Some(else_expr) = else_expr {
                collect_slot_refs(else_expr, slot_ids);
            }
        },
        Expression::InList { expr, list, .. } => {
            collect_slot_refs(expr, slot_ids);
            for item in list {
                collect_slot_refs(item, slot_ids);
            }
        },
        Expression::CorrelatedRef { .. }
        | Expression::Literal(_)
        | Expression::Exists { .. }
        | Expression::ScalarSubquery(_)
        | Expression::Placeholder => {},
    }
}

fn contains_ungrouped_slot_outside_aggregate(
    expr: &Expression,
    grouped_slots: &HashSet<u32>,
    in_aggregate: bool,
) -> bool {
    match expr {
        Expression::SlotRef(slot_id) => !in_aggregate && !grouped_slots.contains(slot_id),
        Expression::CorrelatedRef { .. }
        | Expression::Literal(_)
        | Expression::Exists { .. }
        | Expression::ScalarSubquery(_)
        | Expression::Placeholder => false,
        Expression::BinaryOp { left, right, .. } => {
            contains_ungrouped_slot_outside_aggregate(left, grouped_slots, in_aggregate)
                || contains_ungrouped_slot_outside_aggregate(right, grouped_slots, in_aggregate)
        },
        Expression::UnaryOp { expr, .. }
        | Expression::Cast { expr, .. }
        | Expression::IsNull { expr, .. }
        | Expression::InSubquery { expr, .. } => {
            contains_ungrouped_slot_outside_aggregate(expr, grouped_slots, in_aggregate)
        },
        Expression::Function { args, .. } => args
            .iter()
            .any(|arg| contains_ungrouped_slot_outside_aggregate(arg, grouped_slots, in_aggregate)),
        Expression::AggregateCall { .. } => false,
        Expression::WindowCall {
            args,
            partition_by,
            order_by,
            ..
        } => {
            args.iter().any(|arg| {
                contains_ungrouped_slot_outside_aggregate(arg, grouped_slots, in_aggregate)
            }) || partition_by.iter().any(|expr| {
                contains_ungrouped_slot_outside_aggregate(expr, grouped_slots, in_aggregate)
            }) || order_by.iter().any(|key| {
                contains_ungrouped_slot_outside_aggregate(&key.expr, grouped_slots, in_aggregate)
            })
        },
        Expression::Case {
            operand,
            when_clauses,
            else_expr,
        } => {
            operand.as_ref().is_some_and(|value| {
                contains_ungrouped_slot_outside_aggregate(value, grouped_slots, in_aggregate)
            }) || when_clauses.iter().any(|(condition, result)| {
                contains_ungrouped_slot_outside_aggregate(condition, grouped_slots, in_aggregate)
                    || contains_ungrouped_slot_outside_aggregate(
                        result,
                        grouped_slots,
                        in_aggregate,
                    )
            }) || else_expr.as_ref().is_some_and(|value| {
                contains_ungrouped_slot_outside_aggregate(value, grouped_slots, in_aggregate)
            })
        },
        Expression::InList { expr, list, .. } => {
            contains_ungrouped_slot_outside_aggregate(expr, grouped_slots, in_aggregate)
                || list.iter().any(|item| {
                    contains_ungrouped_slot_outside_aggregate(item, grouped_slots, in_aggregate)
                })
        },
    }
}
