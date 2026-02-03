use sqlex_common::ColumnInfo;

use crate::{
    planner::plan::{CTEContext, LogicalNode, PlanNode, TypedExpr},
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct FilterNode {
    pub input: Box<dyn PlanNode>,
    pub predicate: Box<TypedExpr>,
}

impl LogicalNode for FilterNode {
    fn columns(&self, schema: &Schema, cte_ctx: &CTEContext) -> Vec<ColumnInfo> {
        self.input.columns(schema, cte_ctx)
    }
}
