use crate::{
    planner::plan::{CTEContext, LogicalNode, PlanNodeColumn, TypedExpr},
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct ValuesNode {
    pub rows: Vec<Vec<TypedExpr>>,
    pub column_names: Vec<String>,
}

impl LogicalNode for ValuesNode {
    fn columns(&self, _schema: &Schema, _ctx: &CTEContext) -> Vec<PlanNodeColumn> {
        if let Some(first_row) = self.rows.first() {
            first_row
                .iter()
                .enumerate()
                .map(|(i, expr)| {
                    let name = self
                        .column_names
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| format!("column{}", i + 1));
                    // Check if any row has NULL at this position
                    let nullable = self
                        .rows
                        .iter()
                        .any(|r| r.get(i).map(|e| e.nullable).unwrap_or(true));
                    PlanNodeColumn {
                        name,
                        data_type: expr.data_type.clone(),
                        nullability: nullable,
                        origin_table: None,
                        origin_column: None,
                    }
                })
                .collect()
        } else {
            vec![]
        }
    }
}
