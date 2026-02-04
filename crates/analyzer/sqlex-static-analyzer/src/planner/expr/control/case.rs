use sqlex_analyzer::Result;
use sqlex_common::DataType;

use crate::planner::{
    BuildContext,
    expr::{Expression, ExpressionNode},
    scope::Scope,
};

/// CASE expression
#[derive(Debug, Clone)]
pub struct CaseExpr {
    /// Optional operand (for simple CASE)
    pub operand: Option<Box<dyn Expression>>,
    /// Condition expressions (WHEN clauses)
    pub conditions: Vec<Box<dyn Expression>>,
    /// Result expressions (THEN clauses)
    pub results: Vec<Box<dyn Expression>>,
    /// ELSE expression
    pub else_result: Option<Box<dyn Expression>>,
    /// Inferred return type
    pub return_type: DataType,
    /// Whether the result can be null
    pub is_nullable: bool,
}

impl ExpressionNode for CaseExpr {
    fn data_type(&self) -> DataType {
        self.return_type.clone()
    }

    fn nullable(&self) -> bool {
        self.is_nullable
    }
}

impl CaseExpr {
    pub fn build(
        ctx: &mut BuildContext,
        operand: &Option<Box<sqlparser::ast::Expr>>,
        conditions: &[sqlparser::ast::Expr],
        results: &[sqlparser::ast::Expr],
        else_result: &Option<Box<sqlparser::ast::Expr>>,
        scope: &Scope,
    ) -> Result<Box<dyn Expression>> {
        let operand_expr = if let Some(op) = operand {
            Some(ctx.build_expr(op, scope)?)
        } else {
            None
        };
        let mut cond_exprs = Vec::new();
        for cond in conditions {
            cond_exprs.push(ctx.build_expr(cond, scope)?);
        }
        let mut result_exprs = Vec::new();
        for res in results {
            result_exprs.push(ctx.build_expr(res, scope)?);
        }
        let else_expr = if let Some(el) = else_result {
            Some(ctx.build_expr(el, scope)?)
        } else {
            None
        };

        // Return type is the type of the first THEN clause
        let return_type = result_exprs
            .first()
            .map(|r| r.data_type())
            .unwrap_or(DataType::Custom("unknown".to_string()));

        // Nullability: if no ELSE, or any branch is nullable
        let is_nullable = if else_expr.is_none() {
            true
        } else {
            result_exprs.iter().any(|r| r.nullable())
                || else_expr.as_ref().map(|e| e.nullable()).unwrap_or(false)
        };

        Ok(Box::new(CaseExpr {
            operand: operand_expr,
            conditions: cond_exprs,
            results: result_exprs,
            else_result: else_expr,
            return_type,
            is_nullable,
        }))
    }
}
