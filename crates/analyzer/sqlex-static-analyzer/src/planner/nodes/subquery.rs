use sqlex_common::ColumnInfo;

use crate::{
    planner::plan::{CTEContext, LogicalNode, PlanNode},
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct SubqueryNode {
    pub query: Box<dyn PlanNode>,
    pub alias: String,
}

impl LogicalNode for SubqueryNode {
    fn columns(&self, schema: &Schema, ctx: &CTEContext) -> Vec<ColumnInfo> {
        self.query.columns(schema, ctx)
    }
}
