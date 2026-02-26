use crate::node::expression::Expression;

#[derive(Debug, Clone)]
pub struct ValuesRelation {
    pub rows: Vec<Vec<Expression>>,
}

impl ValuesRelation {
    pub fn new(rows: Vec<Vec<Expression>>) -> Self {
        Self { rows }
    }
}
