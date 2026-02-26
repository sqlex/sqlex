use crate::{arena::RelationId, node::expression::Expression};

#[derive(Debug, Clone)]
pub struct SortKey {
    pub expr: Expression,
    pub asc: bool,
    pub nulls_first: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct SortRelation {
    pub input: RelationId,
    pub keys: Vec<SortKey>,
}

impl SortRelation {
    pub fn new(input: RelationId, keys: Vec<SortKey>) -> Self {
        Self { input, keys }
    }
}
