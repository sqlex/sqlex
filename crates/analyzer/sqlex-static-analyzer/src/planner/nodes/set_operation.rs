use sqlex_common::ColumnInfo;

use crate::{
    planner::plan::{CTEContext, LogicalNode, PlanNode, SetOp},
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct SetOperationNode {
    pub op: SetOp,
    pub all: bool,
    pub left: Box<dyn PlanNode>,
    pub right: Box<dyn PlanNode>,
}

impl LogicalNode for SetOperationNode {
    fn columns(&self, schema: &Schema, ctx: &CTEContext) -> Vec<ColumnInfo> {
        // Use left side's columns (SQL standard: names come from left)
        self.left.columns(schema, ctx)
    }
}
