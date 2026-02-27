use sqlex_analyzer::error::AnalyzerError;

use crate::{
    algebraizer::model::relation::DistinctNode,
    infer::{
        Inferencer,
        model::metadata::InferMetadata,
        relation::{key::slots_key, schema::align_columns_to_schema},
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_distinct_relation(
        &mut self,
        node: &DistinctNode,
    ) -> Result<InferMetadata, AnalyzerError> {
        let mut child = self.infer_relation(&node.input)?;
        let output_columns = align_columns_to_schema(&child.columns, &node.schema);
        child.keys = slots_key(output_columns.iter().filter_map(|column| column.slot_id));

        Ok(InferMetadata {
            columns: output_columns,
            cardinality: child.cardinality,
            keys: child.keys,
        })
    }
}
