use crate::node::expression::Expression;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Eq,
    NotEq,
    Lt,
    Lte,
    Gt,
    Gte,
    Add,
    Sub,
    Mul,
    Div,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub struct BinaryOpExpression {
    pub left: Box<Expression>,
    pub op: BinaryOp,
    pub right: Box<Expression>,
}

impl BinaryOpExpression {
    pub fn new(left: Expression, op: BinaryOp, right: Expression) -> Self {
        Self {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }
}
