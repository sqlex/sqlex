use crate::planner::{
    expr::TypedExpr,
    plan::{LogicalNode, PlanNodeColumn},
};

#[derive(Debug, Clone)]
pub struct ValuesNode {
    pub rows: Vec<Vec<TypedExpr>>,
    pub column_names: Vec<String>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl ValuesNode {
    pub fn build(rows: Vec<Vec<TypedExpr>>, column_names: Vec<String>) -> Self {
        let mut output_columns: Vec<PlanNodeColumn> = vec![];

        if let Some(first_row) = rows.first() {
            output_columns = first_row
                .iter()
                .enumerate()
                .map(|(i, expr)| {
                    let name = column_names
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| format!("column{}", i + 1));
                    // Check if any row has NULL at this position
                    let nullable = rows
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
                .collect();
        }

        Self {
            rows,
            column_names,
            output_columns,
        }
    }
}

impl LogicalNode for ValuesNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
