use sqlex_analyzer::Result;

use super::typed::TypedExpr;
use crate::planner::scope::Scope;

/// ORDER BY expression
#[derive(Debug, Clone)]
pub struct OrderByExpr {
    pub expr: TypedExpr,
    pub asc: bool,
    pub nulls_first: Option<bool>,
}

impl OrderByExpr {
    /// Build from AST OrderByExpr
    pub fn from_ast(ast_order_by: &sqlparser::ast::OrderByExpr, scope: &Scope) -> Result<Self> {
        let expr = TypedExpr::from_expr(&ast_order_by.expr, scope)?;
        Ok(Self {
            expr,
            asc: ast_order_by.asc.unwrap_or(true),
            nulls_first: ast_order_by.nulls_first,
        })
    }
}
