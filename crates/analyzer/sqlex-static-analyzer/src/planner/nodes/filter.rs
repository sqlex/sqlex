use crate::planner::plan::{LogicalNode, PlanNode, PlanNodeColumn, TypedExpr};

#[derive(Debug, Clone)]
pub struct FilterNode {
    pub input: Box<dyn PlanNode>,
    pub predicate: Box<TypedExpr>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl FilterNode {
    pub fn build(input: Box<dyn PlanNode>, predicate: Box<TypedExpr>) -> Self {
        let output_columns = input.columns().to_vec();
        Self {
            input,
            predicate,
            output_columns,
        }
    }
}

impl LogicalNode for FilterNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
