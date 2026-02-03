use crate::{
    planner::plan::{CTEContext, LogicalNode, PlanNode, PlanNodeColumn, TypedExpr},
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct DistinctOnNode {
    pub input: Box<dyn PlanNode>,
    pub on_exprs: Vec<TypedExpr>,
}

impl LogicalNode for DistinctOnNode {
    fn columns(&self, schema: &Schema, ctx: &CTEContext) -> Vec<PlanNodeColumn> {
        self.input.columns(schema, ctx)
    }
}
