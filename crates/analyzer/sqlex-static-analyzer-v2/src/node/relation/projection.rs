use crate::{arena::RelationId, node::expression::Expression};

#[derive(Debug, Clone)]
pub struct ProjectionRelation {
    pub input: RelationId,
    pub expressions: Vec<Expression>,
}

impl ProjectionRelation {
    pub fn new(input: RelationId, expressions: Vec<Expression>) -> Self {
        Self { input, expressions }
    }
}
