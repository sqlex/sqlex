use crate::planner::plan::{LogicalNode, PlanNodeColumn};

#[derive(Debug, Clone)]
pub struct CTERefNode {
    pub name: String,
    pub alias: Option<String>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl CTERefNode {
    pub fn build(
        name: String,
        alias: Option<String>,
        mut output_columns: Vec<PlanNodeColumn>,
    ) -> Self {
        // Update origin table if alias is provided, or just to reflect this CTE usage
        let origin_name = alias.as_ref().unwrap_or(&name).clone();
        for col in &mut output_columns {
            col.origin_table = Some(origin_name.clone());
        }

        Self {
            name,
            alias,
            output_columns,
        }
    }
}

impl LogicalNode for CTERefNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
