use crate::planner::{
    expr::OrderByExpr,
    plan::{LogicalNode, PlanNode, PlanNodeColumn},
};

#[derive(Debug, Clone)]
pub struct SortNode {
    pub input: Box<dyn PlanNode>,
    pub order_by: Vec<OrderByExpr>,
}

impl SortNode {
    pub fn build(input: Box<dyn PlanNode>, order_by: Vec<OrderByExpr>) -> Self {
        Self { input, order_by }
    }
}

impl LogicalNode for SortNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        self.input.columns()
    }
}
