use sqlex_analyzer::error::AnalyzerError;

use crate::{
    algebraizer::model::relation::AggregationNode,
    infer::{
        Inferencer,
        model::{cardinality::CardInterval, metadata::InferMetadata},
        relation::schema::align_columns_to_schema,
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_aggregation_relation(
        &mut self,
        node: &AggregationNode,
    ) -> Result<InferMetadata, AnalyzerError> {
        let child = self.infer_relation(&node.input)?;
        for projection in &node.group_by {
            let _ = self.infer_expression(&projection.expr, &child.columns)?;
        }
        for projection in &node.aggregates {
            let _ = self.infer_expression(&projection.expr, &child.columns)?;
        }

        let columns = align_columns_to_schema(&child.columns, &node.schema);
        let cardinality = if node.group_by.is_empty() && !node.aggregates.is_empty() {
            CardInterval::exactly_one()
        } else {
            child.cardinality
        };

        Ok(InferMetadata {
            columns,
            cardinality,
            keys: Vec::new(),
        })
    }
}
