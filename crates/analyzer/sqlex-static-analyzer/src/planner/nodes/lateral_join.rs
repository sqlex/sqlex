use crate::planner::plan::{JoinKind, LogicalNode, PlanNode, PlanNodeColumn};

#[derive(Debug, Clone)]
pub struct LateralJoinNode {
    pub left: Box<dyn PlanNode>,
    pub lateral: Box<dyn PlanNode>,
    pub kind: JoinKind,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl LateralJoinNode {
    pub fn build(left: Box<dyn PlanNode>, lateral: Box<dyn PlanNode>, kind: JoinKind) -> Self {
        let mut output_columns = left.columns().to_vec();
        let mut right_cols = lateral.columns().to_vec();

        if matches!(kind, JoinKind::Left) {
            for col in &mut right_cols {
                col.nullability = true;
            }
        }

        output_columns.extend(right_cols);

        Self {
            left,
            lateral,
            kind,
            output_columns,
        }
    }
}

impl LogicalNode for LateralJoinNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
