use crate::{
    planner::plan::{CTEContext, LogicalNode, OrderByExpr, PlanNode, PlanNodeColumn},
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct SortNode {
    pub input: Box<dyn PlanNode>,
    pub order_by: Vec<OrderByExpr>,
}

impl LogicalNode for SortNode {
    fn columns(&self, schema: &Schema, ctx: &CTEContext) -> Vec<PlanNodeColumn> {
        self.input.columns(schema, ctx)
    }
}
