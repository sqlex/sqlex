use sqlex_analyzer::error::AnalyzerError;

use crate::{
    algebraizer::model::relation::SortNode,
    infer::{
        Inferencer, model::metadata::InferMetadata, relation::schema::align_columns_to_schema,
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_sort_relation(
        &mut self,
        node: &SortNode,
    ) -> Result<InferMetadata, AnalyzerError> {
        let child = self.infer_relation(&node.input)?;
        for key in &node.keys {
            let _ = self.infer_expression(&key.expr, &child.columns)?;
        }
        Ok(InferMetadata {
            columns: align_columns_to_schema(&child.columns, &node.schema),
            cardinality: child.cardinality,
            keys: child.keys,
        })
    }
}
