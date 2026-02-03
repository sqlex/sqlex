use sqlex_common::ColumnInfo;

use crate::{
    planner::plan::{CTEContext, LogicalNode},
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct CTERefNode {
    pub name: String,
    pub alias: Option<String>,
}

impl LogicalNode for CTERefNode {
    fn columns(&self, _schema: &Schema, ctx: &CTEContext) -> Vec<ColumnInfo> {
        ctx.get(&self.name).cloned().unwrap_or_default()
    }
}
