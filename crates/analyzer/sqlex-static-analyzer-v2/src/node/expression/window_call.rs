use crate::node::expression::Expression;

#[derive(Debug, Clone)]
pub struct WindowOrderKey {
    pub expr: Expression,
    pub asc: bool,
    pub nulls_first: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct WindowCallExpression {
    pub name: String,
    pub args: Vec<Expression>,
    pub partition_by: Vec<Expression>,
    pub order_by: Vec<WindowOrderKey>,
}

impl WindowCallExpression {
    pub fn new(
        name: impl Into<String>,
        args: Vec<Expression>,
        partition_by: Vec<Expression>,
        order_by: Vec<WindowOrderKey>,
    ) -> Self {
        Self {
            name: name.into(),
            args,
            partition_by,
            order_by,
        }
    }
}
