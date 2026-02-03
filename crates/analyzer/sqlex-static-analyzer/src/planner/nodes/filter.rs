use crate::planner::{
    expr::TypedExpr,
    plan::{LogicalNode, PlanNode, PlanNodeColumn},
};

#[derive(Debug, Clone)]
pub struct FilterNode {
    pub input: Box<dyn PlanNode>,
    pub predicate: Box<TypedExpr>,
}

impl FilterNode {
    pub fn build(input: Box<dyn PlanNode>, predicate: Box<TypedExpr>) -> Self {
        Self { input, predicate }
    }
}

impl LogicalNode for FilterNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        self.input.columns()
    }
}
