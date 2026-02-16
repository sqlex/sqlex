use crate::{
    algebraizer::model::relation::{Relation, SelectionNode},
    diagnostics::Diagnostic,
    infer::{
        Inferencer,
        model::{cardinality::CardInterval, metadata::InferColumn},
        relation::predicate::{condition_implies_empty_result, selection_is_at_most_one},
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_selection_relation(
        &self,
        node: &SelectionNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<crate::infer::model::metadata::InferMetadata, Diagnostic> {
        let mut child = self.infer_relation_with_outer_scopes(&node.input, outer_scopes)?;

        let _ = self.infer_expression(&node.condition, &child.columns, outer_scopes)?;

        if condition_implies_empty_result(&node.condition, &child.keys, &child.columns) {
            child.cardinality = CardInterval::exactly_zero();
            return Ok(child);
        }

        if let Relation::Join(join_node) = node.input.as_ref() {
            child.cardinality = self.refine_join_cardinality_from_selection(
                child.cardinality,
                join_node,
                &node.condition,
                outer_scopes,
            )?;
        }

        if selection_is_at_most_one(&node.condition, &child.keys, &child.columns) {
            child.cardinality = child.cardinality.constrain_at_most_one();
        }

        Ok(child)
    }
}
