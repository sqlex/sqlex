use crate::planner::plan::{CTEDef, LogicalNode, PlanNode, PlanNodeColumn};

#[derive(Debug, Clone)]
pub struct WithCTENode {
    pub ctes: Vec<CTEDef>,
    pub body: Box<dyn PlanNode>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl WithCTENode {
    pub fn build(ctes: Vec<CTEDef>, body: Box<dyn PlanNode>) -> Self {
        let output_columns = body.columns().to_vec();
        Self {
            ctes,
            body,
            output_columns,
        }
    }
}

impl LogicalNode for WithCTENode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
