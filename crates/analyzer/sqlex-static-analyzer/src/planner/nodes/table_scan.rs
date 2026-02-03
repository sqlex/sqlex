use crate::{
    planner::plan::{CTEContext, LogicalNode, PlanNodeColumn},
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct TableScanNode {
    pub table: String,
    pub alias: Option<String>,
}

impl LogicalNode for TableScanNode {
    fn columns(&self, schema: &Schema, _ctx: &CTEContext) -> Vec<PlanNodeColumn> {
        schema
            .get_table(&self.table)
            .map(|t| {
                t.columns
                    .iter()
                    .map(|c| PlanNodeColumn {
                        name: c.name.clone(),
                        data_type: c.data_type.clone(),
                        nullability: c.nullable,
                        origin_table: self.alias.clone().or_else(|| Some(self.table.clone())),
                        origin_column: Some(c.name.clone()),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}
