use crate::planner::plan::{LogicalNode, PlanNode, PlanNodeColumn, WindowExpr, window_result_type};

#[derive(Debug, Clone)]
pub struct WindowNode {
    pub input: Box<dyn PlanNode>,
    pub functions: Vec<WindowExpr>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl WindowNode {
    pub fn build(input: Box<dyn PlanNode>, functions: Vec<WindowExpr>) -> Self {
        let mut output_columns = input.columns().to_vec();
        for (i, win) in functions.iter().enumerate() {
            let (data_type, _nullable) = window_result_type(&win.function, &win.args);
            output_columns.push(PlanNodeColumn {
                name: format!("window_{}", i),
                data_type,
                nullability: true,
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
