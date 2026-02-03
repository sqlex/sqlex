use crate::{
    planner::plan::{CTEContext, LogicalNode, PlanNode, PlanNodeColumn, ProjectColumn},
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct ProjectNode {
    pub input: Box<dyn PlanNode>,
    pub columns: Vec<ProjectColumn>,
}

impl LogicalNode for ProjectNode {
    fn columns(&self, _schema: &Schema, _ctx: &CTEContext) -> Vec<PlanNodeColumn> {
        self.columns
            .iter()
            .enumerate()
            .map(|(i, c)| PlanNodeColumn {
                name: c.alias.clone().unwrap_or_else(|| format!("col_{}", i)),
                data_type: c.expr.data_type.clone(),
                nullability: c.expr.nullable,
                origin_table: None, // Projection mostly obscures origin unless we track it
                origin_column: None,
            })
            .collect()
    }
}
