use crate::planner::{
    expr::TypedExpr,
    plan::{LogicalNode, PlanNode, PlanNodeColumn},
};

/// Project column (SELECT item)
#[derive(Debug, Clone)]
pub struct ProjectColumn {
    pub alias: Option<String>,
    pub expr: TypedExpr,
}

#[derive(Debug, Clone)]
pub struct ProjectNode {
    pub input: Box<dyn PlanNode>,
    pub columns: Vec<ProjectColumn>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl ProjectNode {
    pub fn build(input: Box<dyn PlanNode>, columns: Vec<ProjectColumn>) -> Self {
        let output_columns = columns
            .iter()
            .enumerate()
            .map(|(i, col)| PlanNodeColumn {
                name: col.alias.clone().unwrap_or_else(|| format!("col_{}", i)),
                data_type: col.expr.data_type.clone(),
                nullability: col.expr.nullable,
                origin_table: None, // Projection mostly obscures origin unless we track it
                origin_column: None,
            })
            .collect();

        Self {
            input,
            columns,
            output_columns,
        }
    }
}

impl LogicalNode for ProjectNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
