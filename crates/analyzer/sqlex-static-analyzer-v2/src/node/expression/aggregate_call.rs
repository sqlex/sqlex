use crate::node::expression::Expression;

#[derive(Debug, Clone)]
pub struct AggregateCallExpression {
    pub name: String,
    pub args: Vec<Expression>,
    pub distinct: bool,
}

impl AggregateCallExpression {
    pub fn new(name: impl Into<String>, args: Vec<Expression>, distinct: bool) -> Self {
        Self {
            name: name.into(),
            args,
            distinct,
        }
    }
}
