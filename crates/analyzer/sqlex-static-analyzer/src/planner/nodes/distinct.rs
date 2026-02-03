use crate::{
    planner::plan::{CTEContext, LogicalNode, PlanNode, PlanNodeColumn},
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct DistinctNode {
    pub input: Box<dyn PlanNode>,
}

impl LogicalNode for DistinctNode {
    fn columns(&self, schema: &Schema, ctx: &CTEContext) -> Vec<PlanNodeColumn> {
        self.input.columns(schema, ctx)
    }
}
