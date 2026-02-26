use crate::arena::RelationId;

#[derive(Debug, Clone)]
pub struct ExistsExpression {
    pub subquery: RelationId,
    pub negated: bool,
}

impl ExistsExpression {
    pub fn new(subquery: RelationId, negated: bool) -> Self {
        Self { subquery, negated }
    }
}
