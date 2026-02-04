use sqlex_analyzer::{AnalyzerError, Result};

use crate::planner::{
    BuildContext,
    expr::Expression,
    plan::{LogicalNode, PlanNode, PlanNodeColumn},
    scope::Scope,
};

#[derive(Debug, Clone)]
pub struct ValuesNode {
    pub rows: Vec<Vec<Box<dyn Expression>>>,
    pub column_names: Vec<String>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl ValuesNode {
    pub(crate) fn new(rows: Vec<Vec<Box<dyn Expression>>>, column_names: Vec<String>) -> Self {
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
                        .any(|r| r.get(i).map(|e| e.nullable()).unwrap_or(true));
                    PlanNodeColumn {
                        name,
                        data_type: expr.data_type(),
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

    pub fn build(
        ctx: &mut BuildContext,
        values: &sqlparser::ast::Values,
        scope: &Scope,
    ) -> Result<Box<dyn PlanNode>> {
        let mut rules_rows = Vec::new();
        let mut num_cols = 0;

        for (row_idx, row) in values.rows.iter().enumerate() {
            let mut typed_row = Vec::new();
            for expr in row {
                typed_row.push(ctx.build_expr(expr, scope)?);
            }

            if row_idx == 0 {
                num_cols = typed_row.len();
            } else if typed_row.len() != num_cols {
                return Err(AnalyzerError::AnalysisError(format!(
                    "VALUES clause has mismatched column counts: row {} has {}, expected {}",
                    row_idx + 1,
                    typed_row.len(),
                    num_cols
                )));
            }

            rules_rows.push(typed_row);
        }

        if num_cols == 0 {
            return Err(AnalyzerError::AnalysisError(
                "VALUES clause must have at least one column".to_string(),
            ));
        }

        // Generate default column names: column1, column2, ...
        let column_names = (1..=num_cols).map(|i| format!("column{}", i)).collect();

        Ok(Box::new(Self::new(rules_rows, column_names)))
    }
}

impl LogicalNode for ValuesNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
