use sqlex_common::ColumnInfo;

use crate::{
    planner::plan::{CTEContext, LogicalNode, PlanNode, WindowExpr, window_result_type},
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct WindowNode {
    pub input: Box<dyn PlanNode>,
    pub functions: Vec<WindowExpr>,
}

impl LogicalNode for WindowNode {
    fn columns(&self, schema: &Schema, ctx: &CTEContext) -> Vec<ColumnInfo> {
        let mut cols = self.input.columns(schema, ctx);

        for (i, win) in self.functions.iter().enumerate() {
            let (data_type, _nullable) = window_result_type(&win.function, &win.args);
            cols.push(ColumnInfo {
                name: format!("window_{}", i),
                data_type,
                nullability: true,
                origin_table: None,
                origin_column: None,
            });
        }

        cols
    }
}
