use sqlparser::ast::{SetExpr, SetOperator, SetQuantifier};

use crate::{
    algebra::{
        expr::RelExpr,
        planner::{Algebraizer, context::BuildContext},
        scalar::{BoundColumn, ColumnOrigin, OutputSchema},
    },
    catalog::model::Catalog,
    diagnostics::{Diagnostic, Phase},
    functions::registry::FunctionRegistry,
};

impl Algebraizer {
    pub(crate) fn build_set_expr(
        &self,
        set_expr: &SetExpr,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &mut BuildContext,
    ) -> Result<RelExpr, Diagnostic> {
        match set_expr {
            SetExpr::Select(select) => self.build_select(select, catalog, functions, context),
            SetExpr::Query(query) => {
                let mut nested_context = BuildContext {
                    relation_scopes: context.relation_scopes.clone(),
                    outer_relation_scopes: context.outer_relation_scopes.clone(),
                    next_relation_id: context.next_relation_id,
                    next_slot_id: context.next_slot_id,
                    ctes: context.ctes.clone(),
                    named_windows: std::collections::HashMap::new(),
                    literal_assignment_mode: context.literal_assignment_mode,
                };

                if let Some(with_clause) = &query.with {
                    self.register_ctes(with_clause, catalog, functions, &mut nested_context)?;
                }

                let relation =
                    self.build_set_expr(&query.body, catalog, functions, &mut nested_context)?;
                let relation = self.apply_top_level_order_by(
                    relation,
                    query,
                    catalog,
                    functions,
                    &mut nested_context,
                )?;
                let relation = self.apply_top_level_limit_offset(relation, query)?;

                context.next_relation_id = nested_context.next_relation_id;
                context.next_slot_id = nested_context.next_slot_id;
                Ok(relation)
            },
            SetExpr::SetOperation {
                left,
                op,
                set_quantifier,
                right,
            } => {
                let left_expr = self.build_set_expr(left, catalog, functions, context)?;
                let right_expr = self.build_set_expr(right, catalog, functions, context)?;

                let left_schema = super::output_schema_of(&left_expr)?;
                let right_schema = super::output_schema_of(&right_expr)?;
                if left_schema.columns.len() != right_schema.columns.len() {
                    return Err(Diagnostic::new(
                        "A3019",
                        Phase::Algebraize,
                        format!(
                            "set operation column count mismatch: left {}, right {}",
                            left_schema.columns.len(),
                            right_schema.columns.len()
                        ),
                    ));
                }

                let set_op = match op {
                    SetOperator::Union => crate::algebra::expr::SetOp::Union,
                    SetOperator::Intersect => crate::algebra::expr::SetOp::Intersect,
                    SetOperator::Except | SetOperator::Minus => crate::algebra::expr::SetOp::Except,
                };
                let all = matches!(
                    set_quantifier,
                    SetQuantifier::All | SetQuantifier::AllByName
                );

                let mut columns = Vec::with_capacity(left_schema.columns.len());
                for (left_column, right_column) in
                    left_schema.columns.iter().zip(right_schema.columns.iter())
                {
                    columns.push(BoundColumn {
                        slot_id: context.allocate_slot_id(),
                        name: left_column.name.clone(),
                        table_alias: None,
                        data_type: left_column
                            .data_type
                            .clone()
                            .or_else(|| right_column.data_type.clone()),
                        nullable: left_column.nullable || right_column.nullable,
                        origin: ColumnOrigin::Derived,
                    });
                }

                Ok(RelExpr::SetOperation(crate::algebra::expr::SetOpNode {
                    left: Box::new(left_expr),
                    right: Box::new(right_expr),
                    op: set_op,
                    all,
                    schema: OutputSchema {
                        relation_id: context.allocate_relation_id(),
                        columns,
                    },
                }))
            },
            _ => Err(Diagnostic::new(
                "A3065",
                Phase::Algebraize,
                format!("unsupported set expression in this iteration: {set_expr}"),
            )),
        }
    }
}
