use sqlparser::ast::{Expr, Offset, Value};

use crate::planner::plan::{LogicalNode, PlanNode, PlanNodeColumn};

#[derive(Debug, Clone)]
pub struct LimitNode {
    pub input: Box<dyn PlanNode>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

impl LimitNode {
    /// Build from AST LIMIT and OFFSET expressions
    pub fn from_ast(
        input: Box<dyn PlanNode>,
        limit_expr: Option<&Expr>,
        offset_expr: Option<&Offset>,
    ) -> Self {
        let limit = limit_expr.and_then(Self::parse_limit_expr);
        let offset = offset_expr.and_then(|o| Self::parse_limit_expr(&o.value));

        Self {
            input,
            limit,
            offset,
        }
    }

    /// Parse a constant number from an expression
    fn parse_limit_expr(expr: &Expr) -> Option<u64> {
        match expr {
            Expr::Value(Value::Number(n, _)) => n.parse().ok(),
            _ => None, // Only constant values supported
        }
    }
}

impl LogicalNode for LimitNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        self.input.columns()
    }
}
