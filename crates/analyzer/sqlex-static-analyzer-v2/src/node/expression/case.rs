use crate::node::expression::Expression;

#[derive(Debug, Clone)]
pub struct CaseExpression {
    pub operand: Option<Box<Expression>>,
    pub when_clauses: Vec<(Expression, Expression)>,
    pub else_expr: Option<Box<Expression>>,
}

impl CaseExpression {
    pub fn new(
        operand: Option<Expression>,
        when_clauses: Vec<(Expression, Expression)>,
        else_expr: Option<Expression>,
    ) -> Self {
        Self {
            operand: operand.map(Box::new),
            when_clauses,
            else_expr: else_expr.map(Box::new),
        }
    }
}
