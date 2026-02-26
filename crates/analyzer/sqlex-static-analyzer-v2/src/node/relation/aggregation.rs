use crate::{arena::RelationId, node::expression::Expression};

#[derive(Debug, Clone)]
pub struct AggregationRelation {
    pub input: RelationId,
    pub group_by: Vec<Expression>,
    pub aggregates: Vec<Expression>,
}

impl AggregationRelation {
    pub fn new(input: RelationId, group_by: Vec<Expression>, aggregates: Vec<Expression>) -> Self {
        Self {
            input,
            group_by,
            aggregates,
        }
    }
}
