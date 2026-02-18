use sqlex_common::dialect::Dialect;

use crate::{
    algebraizer::model::expression::BoundBinaryOp,
    infer::{
        Inferencer,
        expression::{ExpressionInference, IntLiteralInfo},
    },
};

impl Inferencer<'_> {
    pub(super) fn fold_int_literal_binary(
        &self,
        op: &BoundBinaryOp,
        left: &ExpressionInference,
        right: &ExpressionInference,
    ) -> Option<IntLiteralInfo> {
        if self.dialect != Dialect::MySQL {
            return None;
        }

        let left_const = left.int_literal_info?;
        let right_const = right.int_literal_info?;
        let value = match op {
            BoundBinaryOp::Add => left_const.value.checked_add(right_const.value)?,
            BoundBinaryOp::Sub => left_const.value.checked_sub(right_const.value)?,
            BoundBinaryOp::Mul => left_const.value.checked_mul(right_const.value)?,
            BoundBinaryOp::Div => return None,
            BoundBinaryOp::Eq
            | BoundBinaryOp::NotEq
            | BoundBinaryOp::Lt
            | BoundBinaryOp::Lte
            | BoundBinaryOp::Gt
            | BoundBinaryOp::Gte
            | BoundBinaryOp::And
            | BoundBinaryOp::Or => return None,
        };

        let unsigned = if value < 0 {
            false
        } else {
            match op {
                BoundBinaryOp::Add | BoundBinaryOp::Mul => {
                    left_const.unsigned || right_const.unsigned
                },
                BoundBinaryOp::Sub => left_const.unsigned && right_const.unsigned,
                BoundBinaryOp::Div
                | BoundBinaryOp::Eq
                | BoundBinaryOp::NotEq
                | BoundBinaryOp::Lt
                | BoundBinaryOp::Lte
                | BoundBinaryOp::Gt
                | BoundBinaryOp::Gte
                | BoundBinaryOp::And
                | BoundBinaryOp::Or => false,
            }
        };

        Some(IntLiteralInfo {
            value,
            unsigned,
            display_width: value.unsigned_abs().to_string().len(),
        })
    }

    pub(super) fn fold_int_literal_negate(
        &self,
        hint: Option<IntLiteralInfo>,
    ) -> Option<IntLiteralInfo> {
        if self.dialect != Dialect::MySQL {
            return None;
        }

        match hint {
            Some(value) => Some(IntLiteralInfo {
                value: value.value.checked_neg()?,
                unsigned: false,
                display_width: value.display_width,
            }),
            None => None,
        }
    }
}
