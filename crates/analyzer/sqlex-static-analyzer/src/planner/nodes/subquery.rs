use crate::planner::plan::{LogicalNode, PlanNode, PlanNodeColumn};

#[derive(Debug, Clone)]
pub struct SubqueryNode {
    pub query: Box<dyn PlanNode>,
    pub alias: String,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl SubqueryNode {
    pub fn build(query: Box<dyn PlanNode>, alias: String) -> Self {
        let mut output_columns = query.columns().to_vec();
        // Update origin table/alias for subquery columns
        for col in &mut output_columns {
            col.origin_table = Some(alias.clone());
        }
        Self {
            query,
            alias,
            output_columns,
        }
    }
}

impl LogicalNode for SubqueryNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
