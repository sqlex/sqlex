use std::collections::HashMap;

use crate::{
    algebraizer::model::{expression::Expression, relation::ProjectionNode},
    diagnostics::{Diagnostic, Phase},
    infer::{
        Inferencer,
        model::metadata::{ColumnOrigin, InferColumn, InferMetadata},
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_projection_relation(
        &self,
        node: &ProjectionNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        let child = self.infer_relation_with_outer_scopes(&node.input, outer_scopes)?;

        let mut columns = Vec::with_capacity(node.columns.len());
        let mut slot_mapping = HashMap::new();
        for (index, projection_column) in node.columns.iter().enumerate() {
            let expression_info =
                self.infer_expression(&projection_column.expr, &child.columns, outer_scopes)?;
            let output_slot_id = node.schema.columns.get(index).map(|column| column.slot_id);
            let output_name = projection_column.alias.clone().ok_or_else(|| {
                Diagnostic::new(
                    "I4201",
                    Phase::Infer,
                    "projection column alias was not assigned during planning",
                )
            })?;
            if let (Expression::SlotRef(input_slot_id), Some(output_slot_id)) =
                (&projection_column.expr, output_slot_id)
            {
                slot_mapping.insert(*input_slot_id, output_slot_id);
            }

            columns.push(InferColumn {
                slot_id: output_slot_id,
                name: output_name,
                data_type: expression_info.data_type,
                nullable: expression_info.nullable,
                origin: ColumnOrigin::Derived,
            });
        }

        Ok(InferMetadata {
            columns,
            cardinality: child.cardinality,
            keys: child.remap_keys(&slot_mapping),
        })
    }
}
