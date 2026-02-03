use crate::planner::plan::{LogicalNode, PlanNode, PlanNodeColumn};

#[derive(Debug, Clone)]
pub struct LimitNode {
    pub input: Box<dyn PlanNode>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl LimitNode {
    pub fn build(input: Box<dyn PlanNode>, limit: Option<u64>, offset: Option<u64>) -> Self {
        let output_columns = input.columns().to_vec();
        Self {
            input,
            limit,
            offset,
            output_columns,
        }
    }
}

impl LogicalNode for LimitNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
