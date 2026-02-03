use crate::planner::plan::{LogicalNode, PlanNode, PlanNodeColumn};

#[derive(Debug, Clone)]
pub struct DistinctNode {
    pub input: Box<dyn PlanNode>,
}

impl DistinctNode {
    pub fn build(input: Box<dyn PlanNode>) -> Self {
        Self { input }
    }
}

impl LogicalNode for DistinctNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        self.input.columns()
    }
}
