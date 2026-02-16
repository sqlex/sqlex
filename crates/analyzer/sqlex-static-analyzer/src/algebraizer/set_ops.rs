use sqlparser::ast::{SetExpr, SetOperator, SetQuantifier};

use crate::{
    algebraizer::{
        Algebraizer,
        model::{
            relation::Relation,
            schema::{BoundColumn, ColumnOrigin, OutputSchema},
        },
    },
    diagnostics::{Diagnostic, Phase},
};

impl Algebraizer<'_> {
    pub(crate) fn build_set_relation(
        &mut self,
        sql_set_expr: &SetExpr,
    ) -> Result<Relation, Diagnostic> {
        match sql_set_expr {
            SetExpr::Select(select) => self.build_select(select),
            SetExpr::Query(query) => {
                let literal_assignment_mode = self.literal_assignment_mode();
                self.build_query_relation(query, literal_assignment_mode)
            },
            SetExpr::SetOperation {
                left,
                op,
                set_quantifier,
                right,
            } => {
                let left_relation = self.build_set_relation(left)?;
                let right_relation = self.build_set_relation(right)?;

                let left_schema = left_relation.output_schema();
                let right_schema = right_relation.output_schema();
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
                    SetOperator::Union => crate::algebraizer::model::relation::SetOp::Union,
                    SetOperator::Intersect => crate::algebraizer::model::relation::SetOp::Intersect,
                    SetOperator::Except | SetOperator::Minus => {
                        crate::algebraizer::model::relation::SetOp::Except
                    },
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
                        slot_id: self.allocate_slot_id(),
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

                Ok(Relation::SetOperation(
                    crate::algebraizer::model::relation::SetOpNode {
                        left: Box::new(left_relation),
                        right: Box::new(right_relation),
                        op: set_op,
                        all,
                        schema: OutputSchema {
                            relation_id: self.allocate_relation_id(),
                            columns,
                        },
                    },
                ))
            },
            _ => Err(Diagnostic::new(
                "A3065",
                Phase::Algebraize,
                format!("unsupported set expression in this iteration: {sql_set_expr}"),
            )),
        }
    }
}
