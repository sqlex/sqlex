use sqlex_common::DataType;
use sqlparser::ast::UnaryOperator;

use crate::planner::expr::{Expression, ExpressionNode};

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
    /// Build a unary expression from a pre-built operand
    pub fn build(op: UnaryOperator, operand: Box<dyn Expression>) -> Box<dyn Expression> {
        let return_type = analyze_unary_type(&op, &operand.data_type());
        let is_nullable = operand.nullable();
        Box::new(UnaryExpr {
            op,
            operand,
            return_type,
            is_nullable,
        })
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
