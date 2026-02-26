use crate::node::expression::Expression;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
    Pos,
}

#[derive(Debug, Clone)]
pub struct UnaryOpExpression {
    pub op: UnaryOp,
    pub expr: Box<Expression>,
}

impl UnaryOpExpression {
    pub fn new(op: UnaryOp, expr: Expression) -> Self {
        Self {
            op,
            expr: Box::new(expr),
        }
    }
}
