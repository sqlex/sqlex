use crate::planner::{
    expr::Expression,
    plan::{LogicalNode, PlanNode, PlanNodeColumn},
};

#[derive(Debug, Clone)]
pub struct FilterNode {
    pub input: Box<dyn PlanNode>,
    pub predicate: Box<dyn Expression>,
}

impl FilterNode {
    pub fn build(input: Box<dyn PlanNode>, predicate: Box<dyn Expression>) -> Self {
        Self { input, predicate }
    }
}

impl LogicalNode for FilterNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        self.input.columns()
    }
}
