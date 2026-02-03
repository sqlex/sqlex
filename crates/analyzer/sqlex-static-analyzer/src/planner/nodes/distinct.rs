use sqlex_common::ColumnInfo;

use crate::{
    planner::plan::{CTEContext, LogicalNode, PlanNode},
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct DistinctNode {
    pub input: Box<dyn PlanNode>,
}

impl LogicalNode for DistinctNode {
    fn columns(&self, schema: &Schema, ctx: &CTEContext) -> Vec<ColumnInfo> {
        self.input.columns(schema, ctx)
    }
}
