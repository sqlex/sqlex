use crate::planner::{
    expr::Expression,
    plan::{LogicalNode, PlanNode, PlanNodeColumn},
};

#[derive(Debug, Clone)]
pub struct DistinctOnNode {
    pub input: Box<dyn PlanNode>,
    pub on_exprs: Vec<Box<dyn Expression>>,
}

impl DistinctOnNode {
    pub fn build(input: Box<dyn PlanNode>, on_exprs: Vec<Box<dyn Expression>>) -> Self {
        Self { input, on_exprs }
    }
}

impl LogicalNode for DistinctOnNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        self.input.columns()
    }
}
