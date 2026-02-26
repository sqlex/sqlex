use crate::node::expression::Expression;

#[derive(Debug, Clone)]
pub struct FunctionExpression {
    pub name: String,
    pub args: Vec<Expression>,
}

impl FunctionExpression {
    pub fn new(name: impl Into<String>, args: Vec<Expression>) -> Self {
        Self {
            name: name.into(),
            args,
        }
    }
}
