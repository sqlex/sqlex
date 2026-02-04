use sqlex_analyzer::Result;
use sqlex_common::DataType;
use sqlparser::ast::UnaryOperator;

use crate::planner::{
    BuildContext,
    expr::{Expression, ExpressionNode},
    scope::Scope,
};

/// Unary operation expression
#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub op: UnaryOperator,
    pub operand: Box<dyn Expression>,
    pub return_type: DataType,
    pub is_nullable: bool,
}

impl ExpressionNode for UnaryExpr {
    fn data_type(&self) -> DataType {
        self.return_type.clone()
    }

    fn nullable(&self) -> bool {
        self.is_nullable
    }
}

impl UnaryExpr {
    pub fn build(
        ctx: &mut BuildContext,
        op: &UnaryOperator,
        expr: &sqlparser::ast::Expr,
        scope: &Scope,
    ) -> Result<Box<dyn Expression>> {
        let operand = ctx.build_expr(expr, scope)?;

        let return_type = analyze_unary_type(op, &operand.data_type());
        let is_nullable = operand.nullable();

        Ok(Box::new(UnaryExpr {
            op: *op,
            operand,
            return_type,
            is_nullable,
        }))
    }
}

/// Infer result type for unary operation
fn analyze_unary_type(op: &UnaryOperator, operand: &DataType) -> DataType {
    match op {
        UnaryOperator::Not => DataType::Bool,
        UnaryOperator::Plus | UnaryOperator::Minus => operand.clone(),
        _ => operand.clone(),
    }
}
