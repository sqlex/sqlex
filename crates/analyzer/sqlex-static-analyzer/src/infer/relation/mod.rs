use crate::{
    algebraizer::model::relation::Relation,
    diagnostics::Diagnostic,
    infer::{
        Inferencer,
        model::{cardinality::CardInterval, metadata::InferMetadata},
    },
};

mod aggregation;
mod alias;
pub(in crate::infer) mod cardinality;
mod distinct;
mod join;
mod key;
mod limit;
mod predicate;
mod projection;
mod scan;
mod schema;
mod selection;
mod set_ops;
mod sort;
mod window;

impl Inferencer<'_> {
    pub(crate) fn infer_relation(
        &mut self,
        relation: &Relation,
    ) -> Result<InferMetadata, Diagnostic> {
        match relation {
            Relation::Scan(node) => self.infer_scan_relation(node),
            Relation::Values(_) => Ok(InferMetadata {
                columns: Vec::new(),
                cardinality: CardInterval::exactly_one(),
                keys: Vec::new(),
            }),
            Relation::Selection(node) => self.infer_selection_relation(node),
            Relation::Aggregation(node) => self.infer_aggregation_relation(node),
            Relation::Window(node) => self.infer_window_relation(node),
            Relation::Projection(node) => self.infer_projection_relation(node),
            Relation::Join(node) => self.infer_join_relation(node),
            Relation::Distinct(node) => self.infer_distinct_relation(node),
            Relation::Sort(node) => self.infer_sort_relation(node),
            Relation::Limit(node) => self.infer_limit_relation(node),
            Relation::Alias(node) => self.infer_alias_relation(node),
            Relation::SetOperation(node) => self.infer_set_operation_relation(node),
        }
    }
}
