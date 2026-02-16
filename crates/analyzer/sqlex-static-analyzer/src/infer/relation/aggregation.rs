use crate::{
    algebraizer::model::relation::AggregationNode,
    diagnostics::Diagnostic,
    infer::{
        Inferencer,
        model::{
            cardinality::CardInterval,
            metadata::{InferColumn, InferMetadata},
        },
        relation::schema::align_columns_to_schema,
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_aggregation_relation(
        &self,
        node: &AggregationNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        let child = self.infer_relation_with_outer_scopes(&node.input, outer_scopes)?;
        for projection in &node.group_by {
            let _ = self.infer_expression(&projection.expr, &child.columns, outer_scopes)?;
        }
        for projection in &node.aggregates {
            let _ = self.infer_expression(&projection.expr, &child.columns, outer_scopes)?;
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
