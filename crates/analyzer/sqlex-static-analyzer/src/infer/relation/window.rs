use crate::{
    algebraizer::model::relation::WindowNode,
    diagnostics::Diagnostic,
    infer::{
        Inferencer,
        model::metadata::{InferColumn, InferMetadata},
        relation::schema::align_columns_to_schema,
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_window_relation(
        &self,
        node: &WindowNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        let child = self.infer_relation_with_outer_scopes(&node.input, outer_scopes)?;
        for projection in &node.window_exprs {
            let _ = self.infer_expression(&projection.expr, &child.columns, outer_scopes)?;
        }

        Ok(InferMetadata {
            columns: align_columns_to_schema(&child.columns, &node.schema),
            cardinality: child.cardinality,
            keys: child.keys,
        })
    }
}
