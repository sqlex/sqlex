use crate::planner::plan::{LogicalNode, PlanNode, PlanNodeColumn};

#[derive(Debug, Clone)]
pub struct DistinctNode {
    pub input: Box<dyn PlanNode>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl DistinctNode {
    pub fn build(input: Box<dyn PlanNode>) -> Self {
        let output_columns = input.columns().to_vec();
        Self {
            input,
            output_columns,
        }
    }
}

impl LogicalNode for DistinctNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
