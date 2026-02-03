use sqlex_common::DataType;
use sqlparser::ast::BinaryOperator;

use crate::planner::expr::{Expression, ExpressionNode};

/// Binary operation expression
#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub left: Box<dyn Expression>,
    pub op: BinaryOperator,
    pub right: Box<dyn Expression>,
    pub return_type: DataType,
    pub is_nullable: bool,
}

impl ExpressionNode for BinaryExpr {
    fn data_type(&self) -> DataType {
        self.return_type.clone()
    }

    fn nullable(&self) -> bool {
        self.is_nullable
    }
}

impl BinaryExpr {
    /// Build a binary expression from pre-built children
    pub fn build(
        left: Box<dyn Expression>,
        op: BinaryOperator,
        right: Box<dyn Expression>,
    ) -> Box<dyn Expression> {
        let return_type = analyze_binary_type(&left.data_type(), &op, &right.data_type());
        let is_nullable = left.nullable() || right.nullable();
        Box::new(BinaryExpr {
            left,
            op,
            right,
            return_type,
            is_nullable,
        })
    }
}

/// Infer result type for binary operation
fn analyze_binary_type(left: &DataType, op: &BinaryOperator, right: &DataType) -> DataType {
    match op {
        // Arithmetic operations: promote to larger type
        BinaryOperator::Plus
        | BinaryOperator::Minus
        | BinaryOperator::Multiply
        | BinaryOperator::Modulo => promote_numeric(left, right),

        // Division usually returns float
        BinaryOperator::Divide => DataType::Double,

        // Comparison operations: return boolean
        BinaryOperator::Gt
        | BinaryOperator::Lt
        | BinaryOperator::GtEq
        | BinaryOperator::LtEq
        | BinaryOperator::Eq
        | BinaryOperator::NotEq => DataType::Bool,

        // Logical operations
        BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Xor => DataType::Bool,

        // String concatenation
        BinaryOperator::StringConcat => DataType::Text,

        // Bitwise operations: return integer
        BinaryOperator::BitwiseOr
        | BinaryOperator::BitwiseAnd
        | BinaryOperator::BitwiseXor
        | BinaryOperator::PGBitwiseShiftLeft
        | BinaryOperator::PGBitwiseShiftRight => promote_numeric(left, right),

        // Other operators: default to left operand type
        _ => left.clone(),
    }
}

/// Promote numeric types to a common type
fn promote_numeric(a: &DataType, b: &DataType) -> DataType {
    use DataType::*;

    match (a, b) {
        (Double, _) | (_, Double) => Double,
        (Float, _) | (_, Float) => Float,
        (Decimal, _) | (_, Decimal) => Decimal,
        (BigInt, _) | (_, BigInt) => BigInt,
        (Int, _) | (_, Int) => Int,
        (SmallInt, _) | (_, SmallInt) => SmallInt,
        (TinyInt, TinyInt) => TinyInt,
        _ => a.clone(),
    }
}
