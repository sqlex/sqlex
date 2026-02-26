use crate::arena::RelationId;

#[derive(Debug, Clone)]
pub struct SelectionRelation {
    pub input: RelationId,
}

impl SelectionRelation {
    pub fn new(input: RelationId) -> Self {
        Self { input }
    }
}
