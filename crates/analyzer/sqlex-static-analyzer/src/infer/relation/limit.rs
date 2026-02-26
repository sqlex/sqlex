use crate::{
    algebraizer::model::relation::LimitNode,
    diagnostics::Diagnostic,
    infer::{
        Inferencer, model::metadata::InferMetadata, relation::cardinality::infer_limit_cardinality,
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_limit_relation(
        &mut self,
        node: &LimitNode,
    ) -> Result<InferMetadata, Diagnostic> {
        let mut child = self.infer_relation(&node.input)?;

        child.cardinality = infer_limit_cardinality(child.cardinality, node.limit, node.offset)?;

        Ok(child)
    }
}
