use crate::node::expression::Expression;

#[derive(Debug, Clone)]
pub struct InListExpression {
    pub expr: Box<Expression>,
    pub list: Vec<Expression>,
    pub negated: bool,
}

impl InListExpression {
    pub fn new(expr: Expression, list: Vec<Expression>, negated: bool) -> Self {
        Self {
            expr: Box::new(expr),
            list,
            negated,
        }
    }
}
