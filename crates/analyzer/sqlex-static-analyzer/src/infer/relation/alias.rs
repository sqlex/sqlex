use crate::{
    algebraizer::model::relation::AliasNode,
    diagnostics::Diagnostic,
    infer::{
        Inferencer,
        model::metadata::{InferColumn, InferMetadata},
        relation::schema::align_columns_to_schema,
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_alias_relation(
        &self,
        node: &AliasNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        let child = self.infer_relation_with_outer_scopes(&node.input, outer_scopes)?;
        Ok(InferMetadata {
            columns: align_columns_to_schema(&child.columns, &node.schema),
            cardinality: child.cardinality,
            keys: child.keys,
        })
    }
}
