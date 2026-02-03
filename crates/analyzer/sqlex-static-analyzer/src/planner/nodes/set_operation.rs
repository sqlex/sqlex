use crate::planner::plan::{LogicalNode, PlanNode, PlanNodeColumn};

/// Set operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOp {
    Union,
    Intersect,
    Except,
}

#[derive(Debug, Clone)]
pub struct SetOperationNode {
    pub op: SetOp,
    pub all: bool,
    pub left: Box<dyn PlanNode>,
    pub right: Box<dyn PlanNode>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl SetOperationNode {
    pub fn build(op: SetOp, all: bool, left: Box<dyn PlanNode>, right: Box<dyn PlanNode>) -> Self {
        // Use left side's columns (SQL standard: names come from left)
        let output_columns = left.columns().to_vec();
        Self {
            op,
            all,
            left,
            right,
            output_columns,
        }
    }
}

impl LogicalNode for SetOperationNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
