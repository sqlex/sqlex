use crate::arena::RelationId;

#[derive(Debug, Clone)]
pub struct DistinctRelation {
    pub input: RelationId,
}

impl DistinctRelation {
    pub fn new(input: RelationId) -> Self {
        Self { input }
    }
}
