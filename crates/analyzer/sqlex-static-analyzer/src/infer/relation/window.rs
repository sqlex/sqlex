use crate::{
    algebraizer::model::relation::WindowNode,
    diagnostics::Diagnostic,
    infer::{
        Inferencer, model::metadata::InferMetadata, relation::schema::align_columns_to_schema,
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_window_relation(
        &mut self,
        node: &WindowNode,
    ) -> Result<InferMetadata, Diagnostic> {
        let child = self.infer_relation(&node.input)?;
        for projection in &node.window_exprs {
            let _ = self.infer_expression(&projection.expr, &child.columns)?;
        }

        Ok(InferMetadata {
            columns: align_columns_to_schema(&child.columns, &node.schema),
            cardinality: child.cardinality,
            keys: child.keys,
        })
    }
}
