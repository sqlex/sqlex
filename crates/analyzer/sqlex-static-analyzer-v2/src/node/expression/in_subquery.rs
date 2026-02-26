use crate::{arena::RelationId, node::expression::Expression};

#[derive(Debug, Clone)]
pub struct InSubqueryExpression {
    pub expr: Box<Expression>,
    pub subquery: RelationId,
    pub negated: bool,
}

impl InSubqueryExpression {
    pub fn new(expr: Expression, subquery: RelationId, negated: bool) -> Self {
        Self {
            expr: Box::new(expr),
            subquery,
            negated,
        }
    }
}
