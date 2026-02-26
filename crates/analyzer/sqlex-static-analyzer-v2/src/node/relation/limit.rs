use crate::arena::RelationId;

#[derive(Debug, Clone)]
pub struct LimitRelation {
    pub input: RelationId,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

impl LimitRelation {
    pub fn new(input: RelationId, limit: Option<u64>, offset: Option<u64>) -> Self {
        Self {
            input,
            limit,
            offset,
        }
    }
}
