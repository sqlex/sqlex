use crate::{algebraizer::model::schema::OutputSchema, infer::model::metadata::InferColumn};

pub(super) fn align_columns_to_schema(
    child_columns: &[InferColumn],
    schema: &OutputSchema,
) -> Vec<InferColumn> {
    if child_columns.len() != schema.columns.len() {
        return child_columns.to_vec();
    }

    child_columns
        .iter()
        .zip(schema.columns.iter())
        .map(|(column, schema_column)| InferColumn {
            slot_id: Some(schema_column.slot_id),
            name: schema_column.name.clone(),
            data_type: column.data_type.clone(),
            nullable: column.nullable,
            origin: column.origin.clone(),
            int_literal_info: column.int_literal_info,
        })
        .collect()
}
