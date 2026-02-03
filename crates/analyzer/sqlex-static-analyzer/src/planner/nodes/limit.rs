use crate::{
    planner::plan::{CTEContext, LogicalNode, PlanNode, PlanNodeColumn},
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct LimitNode {
    pub input: Box<dyn PlanNode>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

impl LogicalNode for LimitNode {
    fn columns(&self, schema: &Schema, ctx: &CTEContext) -> Vec<PlanNodeColumn> {
        self.input.columns(schema, ctx)
    }
}
