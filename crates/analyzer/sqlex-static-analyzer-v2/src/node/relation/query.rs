use crate::arena::RelationId;

#[derive(Debug, Clone)]
pub struct QueryRelation {
    pub ctes: Vec<RelationId>,
    pub body: RelationId,
}

impl QueryRelation {
    pub fn new(ctes: Vec<RelationId>, body: RelationId) -> Self {
        Self { ctes, body }
    }
}
