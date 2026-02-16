use crate::{
    algebraizer::model::relation::LimitNode,
    diagnostics::Diagnostic,
    infer::{
        Inferencer,
        model::metadata::{InferColumn, InferMetadata},
        relation::cardinality::infer_limit_cardinality,
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_limit_relation(
        &self,
        node: &LimitNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        let mut child = self.infer_relation_with_outer_scopes(&node.input, outer_scopes)?;

        child.cardinality = infer_limit_cardinality(child.cardinality, node.limit, node.offset)?;

        Ok(child)
    }
}
