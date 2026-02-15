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
                if query.with.is_some()
                    || query.order_by.is_some()
                    || query.limit.is_some()
                    || query.offset.is_some()
                {
                    return Err(Diagnostic::todo(
                        Phase::Algebraize,
                        "nested query ORDER/LIMIT/WITH planning",
                    ));
                }
                self.build_set_expr(&query.body, catalog, functions, context)
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
            _ => Err(Diagnostic::todo(
                Phase::Algebraize,
                "set operation and values planning",
            )),
        }
    }
}
