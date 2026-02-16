use crate::{
    algebraizer::model::relation::DistinctNode,
    diagnostics::Diagnostic,
    infer::{
        Inferencer,
        model::metadata::{InferColumn, InferMetadata},
        relation::{key::slots_key, schema::align_columns_to_schema},
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_distinct_relation(
        &self,
        node: &DistinctNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        let mut child = self.infer_relation_with_outer_scopes(&node.input, outer_scopes)?;
        let output_columns = align_columns_to_schema(&child.columns, &node.schema);
        child.keys = slots_key(output_columns.iter().filter_map(|column| column.slot_id));

        Ok(InferMetadata {
            columns: output_columns,
            cardinality: child.cardinality,
            keys: child.keys,
        })
    }
}
