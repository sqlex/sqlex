use sqlex_common::ColumnInfo;

use crate::{
    planner::plan::{CTEContext, LogicalNode, OrderByExpr, PlanNode},
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct SortNode {
    pub input: Box<dyn PlanNode>,
    pub order_by: Vec<OrderByExpr>,
}

impl LogicalNode for SortNode {
    fn columns(&self, schema: &Schema, ctx: &CTEContext) -> Vec<ColumnInfo> {
        self.input.columns(schema, ctx)
    }
}
