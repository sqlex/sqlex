use crate::{
    algebraizer::model::relation::{JoinKind, SetOp},
    diagnostics::Diagnostic,
    infer::model::cardinality::{CardInterval, MaxRows, MinRows},
};

pub(in crate::infer) fn infer_set_operation_cardinality(
    op: SetOp,
    _all: bool,
    left: CardInterval,
    right: CardInterval,
) -> Result<CardInterval, Diagnostic> {
    match op {
        SetOp::Union => CardInterval::try_new(
            lower_or(left.min(), right.min()),
            upper_add(left.max(), right.max()),
            "infer_set_operation_cardinality::union",
        ),
        SetOp::Intersect => CardInterval::try_new(
            MinRows::Zero,
            upper_min(left.max(), right.max()),
            "infer_set_operation_cardinality::intersect",
        ),
        SetOp::Except => CardInterval::try_new(
            if matches!(right.max(), MaxRows::Zero) {
                left.min()
            } else {
                MinRows::Zero
            },
            left.max(),
            "infer_set_operation_cardinality::except",
        ),
    }
}

pub(super) fn infer_join_cardinality_without_condition(
    kind: JoinKind,
    left: CardInterval,
    right: CardInterval,
) -> Result<CardInterval, Diagnostic> {
    match kind {
        JoinKind::Inner => CardInterval::try_new(
            MinRows::Zero,
            upper_mul(left.max(), right.max()),
            "infer_join_cardinality_without_condition::inner",
        ),
        JoinKind::Left => {
            let max = if matches!(right.max(), MaxRows::Zero) {
                left.max()
            } else {
                upper_mul(left.max(), right.max())
            };
            CardInterval::try_new(
                left.min(),
                max,
                "infer_join_cardinality_without_condition::left",
            )
        },
        JoinKind::Right => {
            let max = if matches!(left.max(), MaxRows::Zero) {
                right.max()
            } else {
                upper_mul(left.max(), right.max())
            };
            CardInterval::try_new(
                right.min(),
                max,
                "infer_join_cardinality_without_condition::right",
            )
        },
        JoinKind::Full => {
            let max = if matches!(left.max(), MaxRows::Zero) {
                right.max()
            } else if matches!(right.max(), MaxRows::Zero) {
                left.max()
            } else {
                MaxRows::Many
            };
            CardInterval::try_new(
                lower_or(left.min(), right.min()),
                max,
                "infer_join_cardinality_without_condition::full",
            )
        },
        JoinKind::Cross => CardInterval::try_new(
            lower_and(left.min(), right.min()),
            upper_mul(left.max(), right.max()),
            "infer_join_cardinality_without_condition::cross",
        ),
    }
}

pub(in crate::infer) fn infer_limit_cardinality(
    input: CardInterval,
    limit: Option<u64>,
    offset: Option<u64>,
) -> Result<CardInterval, Diagnostic> {
    let has_offset = offset.is_some_and(|value| value > 0);
    match limit {
        Some(0) => Ok(CardInterval::exactly_zero()),
        Some(1) if has_offset => {
            if has_at_most_one_row(input) {
                Ok(CardInterval::exactly_zero())
            } else {
                Ok(CardInterval::at_most_one())
            }
        },
        Some(1) => Ok(input.constrain_at_most_one()),
        _ if has_offset => {
            if has_at_most_one_row(input) {
                Ok(CardInterval::exactly_zero())
            } else {
                Ok(input.drop_lower_bound())
            }
        },
        _ => Ok(input),
    }
}

fn has_at_most_one_row(interval: CardInterval) -> bool {
    matches!(interval.max(), MaxRows::Zero | MaxRows::One)
}

pub(super) fn set_operation_output_nullable(op: SetOp, left: bool, right: bool) -> bool {
    match op {
        SetOp::Union | SetOp::Intersect => left || right,
        SetOp::Except => left,
    }
}

pub(super) fn upper_min(left: MaxRows, right: MaxRows) -> MaxRows {
    match (left, right) {
        (MaxRows::Zero, _) | (_, MaxRows::Zero) => MaxRows::Zero,
        (MaxRows::One, _) | (_, MaxRows::One) => MaxRows::One,
        (MaxRows::Many, MaxRows::Many) => MaxRows::Many,
    }
}

fn upper_add(left: MaxRows, right: MaxRows) -> MaxRows {
    match (left, right) {
        (MaxRows::Zero, MaxRows::Zero) => MaxRows::Zero,
        (MaxRows::Zero, MaxRows::One) | (MaxRows::One, MaxRows::Zero) => MaxRows::One,
        (MaxRows::One, MaxRows::One) => MaxRows::Many,
        _ => MaxRows::Many,
    }
}

fn upper_mul(left: MaxRows, right: MaxRows) -> MaxRows {
    match (left, right) {
        (MaxRows::Zero, _) | (_, MaxRows::Zero) => MaxRows::Zero,
        (MaxRows::One, MaxRows::One) => MaxRows::One,
        _ => MaxRows::Many,
    }
}

fn lower_or(left: MinRows, right: MinRows) -> MinRows {
    if matches!(left, MinRows::One) || matches!(right, MinRows::One) {
        MinRows::One
    } else {
        MinRows::Zero
    }
}

fn lower_and(left: MinRows, right: MinRows) -> MinRows {
    if matches!(left, MinRows::One) && matches!(right, MinRows::One) {
        MinRows::One
    } else {
        MinRows::Zero
    }
}
