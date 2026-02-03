use sqlex_analyzer::Result;
use sqlex_common::DataType;

use crate::planner::expr::{Expression, ExpressionNode};

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
    /// Build a CASE expression
    pub fn build(
        operand: Option<Box<dyn Expression>>,
        conditions: Vec<Box<dyn Expression>>,
        results: Vec<Box<dyn Expression>>,
        else_result: Option<Box<dyn Expression>>,
    ) -> Box<dyn Expression> {
        // Return type is the type of the first THEN clause
        let return_type = results
            .first()
            .map(|r| r.data_type())
            .unwrap_or(DataType::Custom("unknown".to_string()));

        // Nullability: if no ELSE, or any branch is nullable
        let is_nullable = if else_result.is_none() {
            true
        } else {
            results.iter().any(|r| r.nullable())
                || else_result.as_ref().map(|e| e.nullable()).unwrap_or(false)
        };

        Box::new(CaseExpr {
            operand,
            conditions,
            results,
            else_result,
            return_type,
            is_nullable,
        })
    }

    pub fn from_ast<F>(
        operand: &Option<Box<sqlparser::ast::Expr>>,
        conditions: &[sqlparser::ast::Expr],
        results: &[sqlparser::ast::Expr],
        else_result: &Option<Box<sqlparser::ast::Expr>>,
        mut expr_builder: F,
    ) -> Result<Box<dyn Expression>>
    where
        F: FnMut(&sqlparser::ast::Expr) -> Result<Box<dyn Expression>>,
    {
        let operand_expr = if let Some(op) = operand {
            Some(expr_builder(op)?)
        } else {
            None
        };
        let mut cond_exprs = Vec::new();
        for cond in conditions {
            cond_exprs.push(expr_builder(cond)?);
        }
        let mut result_exprs = Vec::new();
        for res in results {
            result_exprs.push(expr_builder(res)?);
        }
        let else_expr = if let Some(el) = else_result {
            Some(expr_builder(el)?)
        } else {
            None
        };
        Ok(Self::build(
            operand_expr,
            cond_exprs,
            result_exprs,
            else_expr,
        ))
    }
}
