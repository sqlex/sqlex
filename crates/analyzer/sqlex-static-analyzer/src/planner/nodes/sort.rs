use sqlex_analyzer::Result;

use crate::planner::{
    expr::OrderByExpr,
    plan::{LogicalNode, PlanNode, PlanNodeColumn},
    scope::Scope,
};

#[derive(Debug, Clone)]
pub struct SortNode {
    pub input: Box<dyn PlanNode>,
    pub order_by: Vec<OrderByExpr>,
}

impl SortNode {
    /// Build from AST ORDER BY expressions
    pub fn from_ast(
        input: Box<dyn PlanNode>,
        ast_order_by_exprs: &[sqlparser::ast::OrderByExpr],
        scope: &Scope,
    ) -> Result<Self> {
        let mut order_by_exprs = Vec::new();
        for ob in ast_order_by_exprs {
            order_by_exprs.push(OrderByExpr::from_ast(ob, scope)?);
        }
        Ok(Self::build(input, order_by_exprs))
    }

    pub fn build(input: Box<dyn PlanNode>, order_by: Vec<OrderByExpr>) -> Self {
        Self { input, order_by }
    }
}

impl LogicalNode for SortNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        self.input.columns()
    }
}
