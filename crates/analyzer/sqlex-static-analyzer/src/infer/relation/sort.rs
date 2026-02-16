use crate::{
    algebraizer::model::relation::SortNode,
    diagnostics::Diagnostic,
    infer::{
        Inferencer,
        model::metadata::{InferColumn, InferMetadata},
        relation::schema::align_columns_to_schema,
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_sort_relation(
        &self,
        node: &SortNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        let child = self.infer_relation_with_outer_scopes(&node.input, outer_scopes)?;
        for key in &node.keys {
            let _ = self.infer_expression(&key.expr, &child.columns, outer_scopes)?;
        }
        Ok(InferMetadata {
            columns: align_columns_to_schema(&child.columns, &node.schema),
            cardinality: child.cardinality,
            keys: child.keys,
        })
    }
}
