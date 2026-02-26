use sqlex_analyzer::extension::data_type_ext::DataTypeExt;
use sqlex_common::types::DataType;

use crate::{
    algebraizer::model::relation::SetOpNode,
    diagnostics::{Diagnostic, Phase},
    infer::{
        Inferencer,
        model::metadata::{ColumnOrigin, InferColumn, InferMetadata},
        relation::cardinality::{infer_set_operation_cardinality, set_operation_output_nullable},
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_set_operation_relation(
        &mut self,
        node: &SetOpNode,
    ) -> Result<InferMetadata, Diagnostic> {
        let left = self.infer_relation(&node.left)?;
        let right = self.infer_relation(&node.right)?;
        if left.columns.len() != right.columns.len() {
            return Err(Diagnostic::new(
                "I4202",
                Phase::Infer,
                format!(
                    "set operation column count mismatch: left {}, right {}",
                    left.columns.len(),
                    right.columns.len()
                ),
            ));
        }

        let mut columns = Vec::with_capacity(left.columns.len());
        for index in 0..left.columns.len() {
            let left_column = &left.columns[index];
            let right_column = &right.columns[index];
            let output_slot_id = node.schema.columns.get(index).map(|column| column.slot_id);
            let output_name = node
                .schema
                .columns
                .get(index)
                .map(|column| column.name.clone())
                .unwrap_or_else(|| left_column.name.clone());
            let data_type = DataType::common_type(
                self.dialect,
                &[
                    left_column.data_type.clone(),
                    right_column.data_type.clone(),
                ],
            )
            .unwrap_or_else(|| left_column.data_type.clone());

            columns.push(InferColumn {
                slot_id: output_slot_id,
                name: output_name,
                data_type,
                nullable: set_operation_output_nullable(
                    node.op.clone(),
                    left_column.nullable,
                    right_column.nullable,
                ),
                origin: ColumnOrigin::Derived,
                int_literal_info: None,
            });
        }

        let cardinality = infer_set_operation_cardinality(
            node.op.clone(),
            node.all,
            left.cardinality,
            right.cardinality,
        )?;

        Ok(InferMetadata {
            columns,
            cardinality,
            keys: Vec::new(),
        })
    }
}
