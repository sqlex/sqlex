use crate::{
    algebraizer::model::relation::{AliasNode, Relation},
    diagnostics::Diagnostic,
    infer::{
        Inferencer, model::metadata::InferMetadata, relation::schema::align_columns_to_schema,
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_alias_relation(
        &mut self,
        node: &AliasNode,
    ) -> Result<InferMetadata, Diagnostic> {
        let narrowing_boundary = !matches!(node.input.as_ref(), Relation::Scan(_));
        let child = self.infer_relation(&node.input)?;

        let child = if narrowing_boundary {
            self.narrow_int_literals_at_boundary(child)
        } else {
            child
        };

        Ok(InferMetadata {
            columns: align_columns_to_schema(&child.columns, &node.schema),
            cardinality: child.cardinality,
            keys: child.keys,
        })
    }
}
