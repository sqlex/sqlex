use sqlex_common::types::DataType;

use crate::{
    algebraizer::model::{relation::ScanNode, schema::ColumnOrigin as BoundColumnOrigin},
    diagnostics::Diagnostic,
    infer::{
        Inferencer,
        model::{
            cardinality::CardInterval,
            metadata::{ColumnOrigin, InferColumn, InferMetadata},
        },
        relation::key::resolve_scan_keys,
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_scan_relation(&self, node: &ScanNode) -> Result<InferMetadata, Diagnostic> {
        let _ = &node.table;
        let mut columns = Vec::with_capacity(node.schema.columns.len());

        for column in &node.schema.columns {
            let data_type = column
                .data_type
                .clone()
                .unwrap_or_else(|| DataType::Custom("unknown".to_string()));
            let origin = match &column.origin {
                BoundColumnOrigin::Base { table, column } => ColumnOrigin::Base {
                    table: table.clone(),
                    column: column.clone(),
                },
                BoundColumnOrigin::Derived => ColumnOrigin::Derived,
            };

            columns.push(InferColumn {
                slot_id: Some(column.slot_id),
                name: column.name.clone(),
                data_type,
                nullable: column.nullable,
                origin,
                int_literal_info: None,
            });
        }

        let keys = resolve_scan_keys(node, self.catalog);

        Ok(InferMetadata {
            columns,
            cardinality: if node.table.starts_with("__recursive_cte__") {
                CardInterval::one_or_more()
            } else {
                CardInterval::zero_or_more()
            },
            keys,
        })
    }
}
