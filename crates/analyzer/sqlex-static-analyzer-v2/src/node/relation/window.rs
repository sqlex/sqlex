use crate::{arena::RelationId, node::expression::Expression};

#[derive(Debug, Clone)]
pub struct WindowRelation {
    pub input: RelationId,
    pub window_exprs: Vec<Expression>,
}

impl WindowRelation {
    pub fn new(input: RelationId, window_exprs: Vec<Expression>) -> Self {
        Self {
            input,
            window_exprs,
        }
    }
}
