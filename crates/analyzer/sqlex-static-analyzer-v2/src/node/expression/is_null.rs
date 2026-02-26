use crate::node::expression::Expression;

#[derive(Debug, Clone)]
pub struct IsNullExpression {
    pub expr: Box<Expression>,
    pub negated: bool,
}

impl IsNullExpression {
    pub fn new(expr: Expression, negated: bool) -> Self {
        Self {
            expr: Box::new(expr),
            negated,
        }
    }
}
