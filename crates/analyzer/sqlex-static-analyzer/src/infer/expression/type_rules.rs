use sqlex_analyzer::extension::data_type_ext::DataTypeExt;
use sqlex_common::{dialect::Dialect, types::DataType};

use crate::{
    algebraizer::model::expression::BoundBinaryOp,
    diagnostics::{Diagnostic, Phase},
};

pub(super) fn validate_binary_op(
    op: &BoundBinaryOp,
    left_type: &DataType,
    right_type: &DataType,
    dialect: Dialect,
) -> Result<(), Diagnostic> {
    let is_arithmetic = matches!(
        op,
        BoundBinaryOp::Add | BoundBinaryOp::Sub | BoundBinaryOp::Mul | BoundBinaryOp::Div
    );
    if !is_arithmetic {
        return Ok(());
    }

    if dialect != Dialect::Postgres {
        return Ok(());
    }

    if is_unknown_type(left_type) || is_unknown_type(right_type) {
        return Ok(());
    }

    if left_type.is_numeric() && right_type.is_numeric() {
        return Ok(());
    }

    Err(Diagnostic::new(
        "I4104",
        Phase::Infer,
        format!(
            "operator '{}' is not defined for {:?} and {:?}",
            binary_op_symbol(op),
            left_type,
            right_type
        ),
    ))
}

pub(super) fn boolean_result_type(dialect: Dialect) -> DataType {
    match dialect {
        Dialect::Postgres => DataType::Bool,
        Dialect::MySQL | Dialect::SQLite => DataType::BigInt,
    }
}

fn binary_op_symbol(op: &BoundBinaryOp) -> &'static str {
    match op {
        BoundBinaryOp::Eq => "=",
        BoundBinaryOp::NotEq => "<>",
        BoundBinaryOp::Lt => "<",
        BoundBinaryOp::Lte => "<=",
        BoundBinaryOp::Gt => ">",
        BoundBinaryOp::Gte => ">=",
        BoundBinaryOp::Add => "+",
        BoundBinaryOp::Sub => "-",
        BoundBinaryOp::Mul => "*",
        BoundBinaryOp::Div => "/",
        BoundBinaryOp::And => "AND",
        BoundBinaryOp::Or => "OR",
    }
}

fn is_unknown_type(data_type: &DataType) -> bool {
    matches!(data_type, DataType::Custom(value) if value == "unknown" || value == "null")
}
