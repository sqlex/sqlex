use std::collections::{HashMap, HashSet};

use sqlex_common::dialect::Dialect;
use sqlparser::ast::{Expr, OrderByExpr, Statement, UnaryOperator, Value};

use crate::{
    algebraizer::{
        context::{BuildContext, RelationScope},
        model::{
            expression::Expression,
            relation::{LimitNode, ProjectionNode, Relation, SortNode},
            schema::{
                BoundColumn, ColumnOrigin, OutputSchema, ProjectionColumn, SortKey, Visibility,
            },
        },
    },
    catalog::Catalog,
    diagnostics::{Diagnostic, Phase},
    functions::FunctionRegistry,
};

pub(crate) mod model;

mod context;
mod cte;
mod expression;
mod from_join;
mod from_table_factor;
mod join;
mod select;
mod set_ops;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Algebraizer {
    dialect: Dialect,
}

impl Algebraizer {
    pub(crate) fn new(dialect: Dialect) -> Self {
        Self { dialect }
    }

    pub(crate) fn build(
        &self,
        statement: &Statement,
        catalog: &Catalog,
        functions: &FunctionRegistry,
    ) -> Result<Relation, Diagnostic> {
        let Statement::Query(query) = statement else {
            return Err(Diagnostic::new(
                "A3001",
                Phase::Algebraize,
                "only query statements are supported in analyze",
            ));
        };

        let mut context = BuildContext::new();
        if let Some(with_clause) = &query.with {
            self.register_ctes(with_clause, catalog, functions, &mut context)?;
        }
        let relational_expr = self.build_set_expr(&query.body, catalog, functions, &mut context)?;
        let relational_expr = self.apply_top_level_order_by(
            relational_expr,
            query,
            catalog,
            functions,
            &mut context,
        )?;
        self.apply_top_level_limit_offset(relational_expr, query)
    }

    fn apply_top_level_order_by(
        &self,
        input_expr: Relation,
        query: &sqlparser::ast::Query,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &mut BuildContext,
    ) -> Result<Relation, Diagnostic> {
        let Some(order_by) = &query.order_by else {
            return Ok(input_expr);
        };

        if order_by.interpolate.is_some() {
            return Err(Diagnostic::new(
                "A3030",
                Phase::Algebraize,
                "ORDER BY INTERPOLATE is not supported in this iteration",
            ));
        }

        let input_schema = output_schema_of(&input_expr)?;
        let order_context = BuildContext {
            relation_scopes: vec![RelationScope {
                visible_names: Vec::new(),
                schema: input_schema.clone(),
                hidden_unqualified_slots: HashSet::new(),
            }],
            outer_relation_scopes: context.outer_relation_scopes.clone(),
            next_relation_id: context.next_relation_id,
            next_slot_id: context.next_slot_id,
            ctes: context.ctes.clone(),
            named_windows: context.named_windows.clone(),
            literal_assignment_mode: false,
        };

        let mut hidden_columns = Vec::new();
        let mut hidden_schema_columns = Vec::new();
        let mut hidden_expr_slots = HashMap::new();
        let disallow_hidden = disallow_hidden_order_by(&input_expr);
        let mut sort_keys = Vec::new();
        for order_expr in &order_by.exprs {
            sort_keys.push(self.bind_top_level_order_key(
                order_expr,
                &input_schema,
                catalog,
                functions,
                &order_context,
                &mut hidden_columns,
                &mut hidden_schema_columns,
                &mut hidden_expr_slots,
                disallow_hidden,
                context,
            )?);
        }

        if hidden_columns.is_empty() {
            let schema = input_schema.clone();
            return Ok(Relation::Sort(SortNode {
                input: Box::new(input_expr),
                keys: sort_keys,
                schema,
            }));
        }

        let mut pre_projection_columns =
            project_all_slots(&input_schema.columns, Visibility::Visible);
        pre_projection_columns.extend(hidden_columns);

        let mut pre_projection_schema_columns = input_schema.columns.clone();
        pre_projection_schema_columns.extend(hidden_schema_columns);
        let pre_projection_schema = OutputSchema {
            relation_id: context.allocate_relation_id(),
            columns: pre_projection_schema_columns,
        };

        let pre_projection_expr = Relation::Projection(ProjectionNode {
            input: Box::new(input_expr),
            columns: pre_projection_columns,
            schema: pre_projection_schema.clone(),
        });
        let sorted_expr = Relation::Sort(SortNode {
            input: Box::new(pre_projection_expr),
            keys: sort_keys,
            schema: pre_projection_schema,
        });

        Ok(Relation::Projection(ProjectionNode {
            input: Box::new(sorted_expr),
            columns: project_all_slots(&input_schema.columns, Visibility::Visible),
            schema: input_schema,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    fn bind_top_level_order_key(
        &self,
        order_expr: &OrderByExpr,
        input_schema: &OutputSchema,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        order_context: &BuildContext,
        hidden_columns: &mut Vec<ProjectionColumn>,
        hidden_schema_columns: &mut Vec<BoundColumn>,
        hidden_expr_slots: &mut HashMap<String, u32>,
        disallow_hidden: bool,
        context: &mut BuildContext,
    ) -> Result<SortKey, Diagnostic> {
        if order_expr.with_fill.is_some() {
            return Err(Diagnostic::new(
                "A3031",
                Phase::Algebraize,
                "ORDER BY WITH FILL is not supported in this iteration",
            ));
        }

        let asc = order_expr.asc.unwrap_or(true);
        let nulls_first = order_expr.nulls_first;
        let bound_expr = if let Some(position) = self.try_parse_order_position(&order_expr.expr)? {
            let Some(column) = input_schema.columns.get(position - 1) else {
                return Err(Diagnostic::new(
                    "A3033",
                    Phase::Algebraize,
                    format!(
                        "ORDER BY position {} is out of range for {} column(s)",
                        position,
                        input_schema.columns.len()
                    ),
                ));
            };
            Expression::SlotRef(column.slot_id)
        } else {
            let (bound_expr, _) =
                self.bind_expression(&order_expr.expr, catalog, functions, order_context)?;
            bound_expr
        };

        let key_expr = match bound_expr {
            Expression::SlotRef(slot_id) => Expression::SlotRef(slot_id),
            other => {
                if disallow_hidden {
                    return Err(Diagnostic::new(
                        "A3049",
                        Phase::Algebraize,
                        "ORDER BY expression must appear in SELECT list when DISTINCT semantics are active",
                    ));
                }

                let fingerprint = bound_expr_key(&other);
                if let Some(existing_slot_id) = hidden_expr_slots.get(&fingerprint) {
                    return Ok(SortKey {
                        expr: Expression::SlotRef(*existing_slot_id),
                        asc,
                        nulls_first,
                    });
                }

                let hidden_slot_id = context.allocate_slot_id();
                let hidden_name = format!("__ord${hidden_slot_id}");
                hidden_columns.push(ProjectionColumn {
                    expr: other,
                    alias: Some(hidden_name.clone()),
                    visibility: Visibility::Hidden,
                });
                hidden_schema_columns.push(BoundColumn {
                    slot_id: hidden_slot_id,
                    name: hidden_name,
                    table_alias: None,
                    data_type: None,
                    nullable: true,
                    origin: ColumnOrigin::Derived,
                });
                hidden_expr_slots.insert(fingerprint, hidden_slot_id);
                Expression::SlotRef(hidden_slot_id)
            },
        };

        Ok(SortKey {
            expr: key_expr,
            asc,
            nulls_first,
        })
    }

    fn try_parse_order_position(&self, expr: &Expr) -> Result<Option<usize>, Diagnostic> {
        match expr {
            Expr::Value(Value::Number(number, _)) => {
                let position = number.parse::<usize>().map_err(|error| {
                    Diagnostic::new(
                        "A3032",
                        Phase::Algebraize,
                        format!("invalid ORDER BY position '{number}': {error}"),
                    )
                })?;
                if position == 0 {
                    return Err(Diagnostic::new(
                        "A3032",
                        Phase::Algebraize,
                        "ORDER BY position starts from 1",
                    ));
                }
                Ok(Some(position))
            },
            Expr::UnaryOp {
                op: UnaryOperator::Plus,
                expr,
            } => self.try_parse_order_position(expr),
            _ => Ok(None),
        }
    }

    fn apply_top_level_limit_offset(
        &self,
        input_expr: Relation,
        query: &sqlparser::ast::Query,
    ) -> Result<Relation, Diagnostic> {
        let limit = query
            .limit
            .as_ref()
            .map(|expr| self.parse_non_negative_integer_expr(expr, "LIMIT"))
            .transpose()?;
        let offset = query
            .offset
            .as_ref()
            .map(|offset| self.parse_non_negative_integer_expr(&offset.value, "OFFSET"))
            .transpose()?;

        if limit.is_none() && offset.is_none() {
            return Ok(input_expr);
        }

        let schema = output_schema_of(&input_expr)?;
        Ok(Relation::Limit(LimitNode {
            input: Box::new(input_expr),
            limit,
            offset,
            schema,
        }))
    }

    fn parse_non_negative_integer_expr(
        &self,
        expr: &Expr,
        clause_name: &str,
    ) -> Result<u64, Diagnostic> {
        match expr {
            Expr::Value(Value::Number(number, _)) => number.parse::<u64>().map_err(|error| {
                Diagnostic::new(
                    "A3018",
                    Phase::Algebraize,
                    format!("invalid {clause_name} value '{number}': {error}"),
                )
            }),
            Expr::UnaryOp {
                op: UnaryOperator::Plus,
                expr,
            } => self.parse_non_negative_integer_expr(expr, clause_name),
            _ => Err(Diagnostic::new(
                "A3050",
                Phase::Algebraize,
                format!("{clause_name} expects a non-negative integer literal"),
            )),
        }
    }
}

fn output_schema_of(expr: &Relation) -> Result<OutputSchema, Diagnostic> {
    match expr {
        Relation::Scan(node) => Ok(node.schema.clone()),
        Relation::Values(node) => Ok(node.schema.clone()),
        Relation::Selection(node) => Ok(node.schema.clone()),
        Relation::Projection(node) => Ok(node.schema.clone()),
        Relation::Aggregation(node) => Ok(node.schema.clone()),
        Relation::Window(node) => Ok(node.schema.clone()),
        Relation::Distinct(node) => Ok(node.schema.clone()),
        Relation::Sort(node) => Ok(node.schema.clone()),
        Relation::Limit(node) => Ok(node.schema.clone()),
        Relation::Alias(node) => Ok(node.schema.clone()),
        Relation::Join(node) => Ok(node.schema.clone()),
        Relation::SetOperation(node) => Ok(node.schema.clone()),
    }
}

fn project_all_slots(columns: &[BoundColumn], visibility: Visibility) -> Vec<ProjectionColumn> {
    columns
        .iter()
        .map(|column| ProjectionColumn {
            expr: Expression::SlotRef(column.slot_id),
            alias: Some(column.name.clone()),
            visibility: visibility.clone(),
        })
        .collect()
}

fn disallow_hidden_order_by(expr: &Relation) -> bool {
    match expr {
        Relation::Distinct(_) => true,
        Relation::SetOperation(node) => !node.all,
        _ => false,
    }
}

fn bound_expr_key(expr: &Expression) -> String {
    format!("{expr:?}")
}
