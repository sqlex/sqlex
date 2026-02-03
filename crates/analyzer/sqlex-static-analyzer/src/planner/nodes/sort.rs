use crate::planner::{
    expr::OrderByExpr,
    plan::{LogicalNode, PlanNode, PlanNodeColumn},
};

#[derive(Debug, Clone)]
pub struct SortNode {
    pub input: Box<dyn PlanNode>,
    pub order_by: Vec<OrderByExpr>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl SortNode {
    pub fn build(input: Box<dyn PlanNode>, order_by: Vec<OrderByExpr>) -> Self {
        let output_columns = input.columns().to_vec();
        Self {
            input,
            order_by,
            output_columns,
        }
    }
}

impl LogicalNode for SortNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
