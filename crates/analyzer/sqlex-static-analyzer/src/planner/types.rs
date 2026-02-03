//! Type inference engine
//!
//! Implements the rules for inferring data types of expressions.

use sqlex_common::DataType;
use sqlparser::ast::{BinaryOperator, UnaryOperator};

use super::expr::AggregateFunction;

/// Infer type for binary operation
pub fn binary_op_type(left: DataType, op: BinaryOperator, right: DataType) -> DataType {
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
        _ => left,
    }
}

/// Infer type for unary operation
pub fn unary_op_type(op: UnaryOperator, operand: DataType) -> DataType {
    match op {
        UnaryOperator::Not => DataType::Bool,
        UnaryOperator::Plus | UnaryOperator::Minus => operand,
        _ => operand,
    }
}

/// Promote numeric types to a common type
pub fn promote_numeric(a: DataType, b: DataType) -> DataType {
    use DataType::*;

    match (&a, &b) {
        // If either is Double, result is Double
        (Double, _) | (_, Double) => Double,

        // If either is Float, result is Float (unless the other is Double)
        (Float, _) | (_, Float) => Float,

        // If either is Decimal, result is Decimal
        (Decimal, _) | (_, Decimal) => Decimal,

        // If either is BigInt, result is BigInt
        (BigInt, _) | (_, BigInt) => BigInt,

        // If either is Int, result is Int
        (Int, _) | (_, Int) => Int,

        // If either is SmallInt, result is SmallInt
        (SmallInt, _) | (_, SmallInt) => SmallInt,

        // TinyInt stays TinyInt
        (TinyInt, TinyInt) => TinyInt,

        // Default to the first type
        _ => a,
    }
}

/// Promote integer type to larger type (for SUM, etc.)
pub fn promote_to_large(t: DataType) -> DataType {
    use DataType::*;

    match t {
        TinyInt | SmallInt | Int => BigInt,
        Float => Double,
        other => other,
    }
}

/// Infer return type of aggregate function
pub fn aggregate_return_type(func: &AggregateFunction, input: DataType) -> DataType {
    match func {
        AggregateFunction::Count => DataType::BigInt,
        AggregateFunction::Sum => promote_to_large(input),
        AggregateFunction::Avg => DataType::Double,
        AggregateFunction::Min | AggregateFunction::Max => input,
        AggregateFunction::ArrayAgg => DataType::Array(Box::new(input)),
        AggregateFunction::StringAgg => DataType::Text,
        AggregateFunction::JsonAgg => DataType::Json,
        AggregateFunction::First | AggregateFunction::Last => input,
        AggregateFunction::Custom(_) => DataType::Custom("unknown".to_string()),
    }
}

/// Map common SQL function names to return types
pub fn function_return_type(name: &str, _args: &[DataType]) -> Option<DataType> {
    let name_upper = name.to_uppercase();
    match name_upper.as_str() {
        // String functions
        "CONCAT" | "CONCAT_WS" | "UPPER" | "LOWER" | "TRIM" | "LTRIM" | "RTRIM" | "SUBSTRING"
        | "SUBSTR" | "REPLACE" | "LEFT" | "RIGHT" | "REPEAT" => Some(DataType::Text),
        "LENGTH" | "CHAR_LENGTH" | "CHARACTER_LENGTH" | "OCTET_LENGTH" | "BIT_LENGTH" => {
            Some(DataType::Int)
        },
        "POSITION" | "STRPOS" => Some(DataType::Int),

        // Numeric functions
        "ABS" | "CEIL" | "CEILING" | "FLOOR" | "ROUND" | "TRUNCATE" | "TRUNC" => None, // Same as input
        "SQRT" | "EXP" | "LOG" | "LN" | "LOG10" | "LOG2" | "POWER" | "POW" => {
            Some(DataType::Double)
        },
        "MOD" => None, // Same as input
        "RANDOM" | "RAND" => Some(DataType::Double),
        "SIGN" => Some(DataType::Int),

        // Date/time functions
        "NOW" | "CURRENT_TIMESTAMP" => Some(DataType::Timestamp),
        "CURRENT_DATE" => Some(DataType::Date),
        "CURRENT_TIME" => Some(DataType::Time),
        "DATE" => Some(DataType::Date),
        "TIME" => Some(DataType::Time),
        "YEAR" | "MONTH" | "DAY" | "HOUR" | "MINUTE" | "SECOND" | "EXTRACT" => Some(DataType::Int),

        // Type conversion
        "CAST" | "CONVERT" => None, // Depends on target type

        // JSON functions
        "JSON_OBJECT" | "JSON_ARRAY" | "TO_JSON" | "TO_JSONB" => Some(DataType::Json),

        // Boolean functions
        "COALESCE" | "NULLIF" | "IFNULL" | "NVL" => None, // Same as first arg

        // Unknown function
        _ => None,
    }
}
