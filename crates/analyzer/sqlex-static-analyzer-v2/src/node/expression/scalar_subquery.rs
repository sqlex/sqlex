use crate::arena::RelationId;

#[derive(Debug, Clone)]
pub struct ScalarSubqueryExpression {
    pub subquery: RelationId,
}

impl ScalarSubqueryExpression {
    pub fn new(subquery: RelationId) -> Self {
        Self { subquery }
    }
}
