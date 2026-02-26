use crate::node::expression::Expression;

#[derive(Debug, Clone)]
pub struct CastExpression {
    pub expr: Box<Expression>,
    pub target_type: String,
}

impl CastExpression {
    pub fn new(expr: Expression, target_type: impl Into<String>) -> Self {
        Self {
            expr: Box::new(expr),
            target_type: target_type.into(),
        }
    }
}
