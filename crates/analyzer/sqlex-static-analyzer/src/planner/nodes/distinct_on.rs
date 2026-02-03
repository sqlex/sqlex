use crate::planner::{
    expr::TypedExpr,
    plan::{LogicalNode, PlanNode, PlanNodeColumn},
};

#[derive(Debug, Clone)]
pub struct DistinctOnNode {
    pub input: Box<dyn PlanNode>,
    pub on_exprs: Vec<TypedExpr>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl DistinctOnNode {
    pub fn build(input: Box<dyn PlanNode>, on_exprs: Vec<TypedExpr>) -> Self {
        let output_columns = input.columns().to_vec();
        Self {
            input,
            on_exprs,
            output_columns,
        }
    }
}

impl LogicalNode for DistinctOnNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
