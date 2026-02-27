use std::collections::HashMap;

use sqlex_analyzer::error::AnalyzerError;
use sqlex_common::types::DataType;

use crate::{
    algebraizer::model::{
        relation::{JoinKind, JoinNode},
        schema::ColumnOrigin as BoundColumnOrigin,
    },
    infer::{
        Inferencer,
        model::metadata::{ColumnOrigin, InferColumn, InferMetadata},
        relation::cardinality::infer_join_cardinality_without_condition,
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_join_relation(
        &mut self,
        node: &JoinNode,
    ) -> Result<InferMetadata, AnalyzerError> {
        let left = self.infer_relation(&node.left)?;
        let right = self.infer_relation(&node.right)?;

        let mut left_columns = left.columns;
        let mut right_columns = right.columns;
        match node.kind {
            JoinKind::Left => {
                for column in &mut right_columns {
                    column.nullable = true;
                }
            },
            JoinKind::Right => {
                for column in &mut left_columns {
                    column.nullable = true;
                }
            },
            JoinKind::Full => {
                for column in &mut left_columns {
                    column.nullable = true;
                }
                for column in &mut right_columns {
                    column.nullable = true;
                }
            },
            JoinKind::Inner | JoinKind::Cross => {},
        }

        let mut columns_by_slot = HashMap::new();
        for column in left_columns.into_iter().chain(right_columns) {
            if let Some(slot_id) = column.slot_id {
                columns_by_slot.insert(slot_id, column);
            }
        }

        let mut columns = Vec::with_capacity(node.schema.columns.len());
        for schema_column in &node.schema.columns {
            if let Some(source_column) = columns_by_slot.get(&schema_column.slot_id) {
                columns.push(InferColumn {
                    slot_id: Some(schema_column.slot_id),
                    name: schema_column.name.clone(),
                    data_type: source_column.data_type.clone(),
                    nullable: source_column.nullable,
                    origin: source_column.origin.clone(),
                    int_literal_info: None,
                });
                continue;
            }

            let data_type = schema_column
                .data_type
                .clone()
                .unwrap_or_else(|| DataType::Custom("unknown".to_string()));
            let origin = match &schema_column.origin {
                BoundColumnOrigin::Base { table, column } => ColumnOrigin::Base {
                    table: table.clone(),
                    column: column.clone(),
                },
                BoundColumnOrigin::Derived => ColumnOrigin::Derived,
            };

            columns.push(InferColumn {
                slot_id: Some(schema_column.slot_id),
                name: schema_column.name.clone(),
                data_type,
                nullable: schema_column.nullable,
                origin,
                int_literal_info: None,
            });
        }

        let cardinality = infer_join_cardinality_without_condition(
            node.kind.clone(),
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
