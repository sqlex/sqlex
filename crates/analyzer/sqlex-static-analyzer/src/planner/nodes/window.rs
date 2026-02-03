use crate::planner::{
    expr::WindowFunctionExpr,
    plan::{LogicalNode, PlanNode, PlanNodeColumn},
};

#[derive(Debug, Clone)]
pub struct WindowNode {
    pub input: Box<dyn PlanNode>,
    pub functions: Vec<WindowFunctionExpr>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl WindowNode {
    pub fn build(input: Box<dyn PlanNode>, functions: Vec<WindowFunctionExpr>) -> Self {
        let mut output_columns = input.columns().to_vec();
        for (i, win) in functions.iter().enumerate() {
            // Using the cached values in WindowFunctionExpr
            output_columns.push(PlanNodeColumn {
                name: format!("window_{}", i),
                data_type: win.return_type.clone(),
                nullability: win.is_nullable,
                origin_table: None,
                origin_column: None,
            });
        }
        Self {
            input,
            functions,
            output_columns,
        }
    }
}

impl LogicalNode for WindowNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
