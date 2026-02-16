use std::collections::{HashMap, HashSet};

use crate::{
    algebraizer::model::{
        expression::{BoundBinaryOp, BoundLiteral, Expression},
        relation::{JoinKind, JoinNode},
    },
    diagnostics::Diagnostic,
    infer::{
        Inferencer,
        model::{
            cardinality::CardInterval,
            metadata::{InferColumn, ResolvedKey},
        },
        relation::cardinality::upper_min,
    },
};

pub(super) fn condition_implies_empty_result(
    condition: &Expression,
    input_keys: &[ResolvedKey],
    input_columns: &[InferColumn],
) -> bool {
    always_false_condition(condition)
        || has_contradictory_equalities(condition)
        || has_full_key_is_null_on_proven_non_nullable_key(condition, input_keys, input_columns)
}

fn has_contradictory_equalities(condition: &Expression) -> bool {
    let mut equalities: HashMap<u32, BoundLiteral> = HashMap::new();
    collect_contradictory_equalities(condition, &mut equalities)
}

fn collect_contradictory_equalities(
    expr: &Expression,
    equalities: &mut HashMap<u32, BoundLiteral>,
) -> bool {
    match expr {
        Expression::BinaryOp { left, op, right } => match op {
            BoundBinaryOp::And => {
                collect_contradictory_equalities(left, equalities)
                    || collect_contradictory_equalities(right, equalities)
            },
            BoundBinaryOp::Eq => {
                equality_constraint_conflicts(left, right, equalities)
                    || equality_constraint_conflicts(right, left, equalities)
            },
            _ => false,
        },
        _ => false,
    }
}

fn equality_constraint_conflicts(
    left: &Expression,
    right: &Expression,
    equalities: &mut HashMap<u32, BoundLiteral>,
) -> bool {
    let Expression::SlotRef(slot_id) = left else {
        return false;
    };
    let Some(literal) = extract_comparable_literal(right) else {
        return false;
    };

    if let Some(existing) = equalities.get(slot_id) {
        return matches!(literal_equal(existing, literal), Some(false));
    }

    equalities.insert(*slot_id, literal.clone());
    false
}

fn extract_comparable_literal(expr: &Expression) -> Option<&BoundLiteral> {
    match expr {
        Expression::Literal(literal) => Some(literal),
        Expression::Cast { expr, .. } => extract_comparable_literal(expr),
        _ => None,
    }
}

fn has_full_key_is_null_on_proven_non_nullable_key(
    condition: &Expression,
    input_keys: &[ResolvedKey],
    input_columns: &[InferColumn],
) -> bool {
    let mut constraints = HashMap::<u32, SingleValueConstraint>::new();
    if !collect_single_value_constraints(condition, &mut constraints) {
        return false;
    }

    input_keys.iter().any(|key| {
        !key.slot_ids.is_empty()
            && key.slot_ids.iter().all(|slot_id| {
                matches!(
                    constraints.get(slot_id),
                    Some(SingleValueConstraint::IsNull)
                ) && column_is_non_nullable(input_columns, *slot_id)
            })
    })
}

fn column_is_non_nullable(columns: &[InferColumn], slot_id: u32) -> bool {
    columns
        .iter()
        .find(|column| column.slot_id.is_some_and(|value| value == slot_id))
        .is_some_and(|column| !column.nullable)
}

impl Inferencer<'_> {
    pub(super) fn refine_join_cardinality_from_selection(
        &self,
        current: CardInterval,
        join_node: &JoinNode,
        condition: &Expression,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<CardInterval, Diagnostic> {
        let left = self.infer_relation_with_outer_scopes(&join_node.left, outer_scopes)?;
        let right = self.infer_relation_with_outer_scopes(&join_node.right, outer_scopes)?;

        let Some(join_pairs) =
            extract_join_equijoin_pairs(condition, &left.columns, &right.columns)
        else {
            return Ok(current);
        };

        let at_most_one_right_per_left =
            join_pairs_cover_non_nullable_key(&join_pairs, &right.keys, &right.columns, false);
        let at_most_one_left_per_right =
            join_pairs_cover_non_nullable_key(&join_pairs, &left.keys, &left.columns, true);

        let mut max = current.max();
        match join_node.kind {
            JoinKind::Inner => {
                if at_most_one_right_per_left {
                    max = upper_min(max, left.cardinality.max());
                }
                if at_most_one_left_per_right {
                    max = upper_min(max, right.cardinality.max());
                }
            },
            JoinKind::Left => {
                if at_most_one_right_per_left {
                    max = upper_min(max, left.cardinality.max());
                }
            },
            JoinKind::Right => {
                if at_most_one_left_per_right {
                    max = upper_min(max, right.cardinality.max());
                }
            },
            JoinKind::Full | JoinKind::Cross => {},
        }

        CardInterval::try_new(current.min(), max, "refine_join_cardinality_from_selection")
    }
}

fn extract_join_equijoin_pairs(
    condition: &Expression,
    left_columns: &[InferColumn],
    right_columns: &[InferColumn],
) -> Option<Vec<(u32, u32)>> {
    let left_slots: HashSet<u32> = left_columns
        .iter()
        .filter_map(|column| column.slot_id)
        .collect();
    let right_slots: HashSet<u32> = right_columns
        .iter()
        .filter_map(|column| column.slot_id)
        .collect();
    if left_slots.is_empty() || right_slots.is_empty() {
        return None;
    }

    let mut pairs = HashSet::new();
    if !collect_join_equijoin_pairs(condition, &left_slots, &right_slots, &mut pairs) {
        return None;
    }

    if pairs.is_empty() {
        None
    } else {
        Some(pairs.into_iter().collect())
    }
}

fn collect_join_equijoin_pairs(
    expr: &Expression,
    left_slots: &HashSet<u32>,
    right_slots: &HashSet<u32>,
    pairs: &mut HashSet<(u32, u32)>,
) -> bool {
    match expr {
        Expression::BinaryOp { left, op, right } => match op {
            BoundBinaryOp::And => {
                collect_join_equijoin_pairs(left, left_slots, right_slots, pairs)
                    && collect_join_equijoin_pairs(right, left_slots, right_slots, pairs)
            },
            BoundBinaryOp::Eq => {
                let (Expression::SlotRef(left_slot), Expression::SlotRef(right_slot)) =
                    (left.as_ref(), right.as_ref())
                else {
                    return false;
                };

                if left_slots.contains(left_slot) && right_slots.contains(right_slot) {
                    pairs.insert((*left_slot, *right_slot));
                    true
                } else if left_slots.contains(right_slot) && right_slots.contains(left_slot) {
                    pairs.insert((*right_slot, *left_slot));
                    true
                } else {
                    false
                }
            },
            _ => false,
        },
        _ => false,
    }
}

fn join_pairs_cover_non_nullable_key(
    join_pairs: &[(u32, u32)],
    keys: &[ResolvedKey],
    columns: &[InferColumn],
    use_left_slot: bool,
) -> bool {
    let constrained_slots: HashSet<u32> = if use_left_slot {
        join_pairs.iter().map(|(left_slot, _)| *left_slot).collect()
    } else {
        join_pairs
            .iter()
            .map(|(_, right_slot)| *right_slot)
            .collect()
    };

    keys.iter().any(|key| {
        !key.slot_ids.is_empty()
            && key
                .slot_ids
                .iter()
                .all(|slot_id| constrained_slots.contains(slot_id))
            && key
                .slot_ids
                .iter()
                .all(|slot_id| column_is_non_nullable(columns, *slot_id))
    })
}

fn always_false_condition(condition: &Expression) -> bool {
    match condition {
        Expression::Literal(BoundLiteral::Bool(value)) => !*value,
        Expression::BinaryOp { left, op, right } => match op {
            BoundBinaryOp::Eq => literal_comparison_false(left, right, true),
            BoundBinaryOp::NotEq => literal_comparison_false(left, right, false),
            _ => false,
        },
        _ => false,
    }
}

fn literal_comparison_false(left: &Expression, right: &Expression, is_eq: bool) -> bool {
    let Expression::Literal(left_literal) = left else {
        return false;
    };
    let Expression::Literal(right_literal) = right else {
        return false;
    };
    let Some(literals_equal) = literal_equal(left_literal, right_literal) else {
        return false;
    };
    if is_eq {
        !literals_equal
    } else {
        literals_equal
    }
}

fn literal_equal(left: &BoundLiteral, right: &BoundLiteral) -> Option<bool> {
    match (left, right) {
        (BoundLiteral::Null, BoundLiteral::Null) => Some(true),
        (BoundLiteral::Bool(left_value), BoundLiteral::Bool(right_value)) => {
            Some(left_value == right_value)
        },
        (
            BoundLiteral::Int {
                value: left_value, ..
            },
            BoundLiteral::Int {
                value: right_value, ..
            },
        ) => Some(left_value == right_value),
        (BoundLiteral::Float(left_value), BoundLiteral::Float(right_value)) => {
            Some(left_value.to_bits() == right_value.to_bits())
        },
        (BoundLiteral::String(left_value), BoundLiteral::String(right_value)) => {
            Some(left_value == right_value)
        },
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SingleValueConstraint {
    EqLike,
    IsNull,
}

pub(super) fn selection_is_at_most_one(
    condition: &Expression,
    input_keys: &[ResolvedKey],
    input_columns: &[InferColumn],
) -> bool {
    let mut constraints = HashMap::<u32, SingleValueConstraint>::new();
    if !collect_single_value_constraints(condition, &mut constraints) {
        return false;
    }

    input_keys
        .iter()
        .any(|key| key_satisfied(key, &constraints, input_columns))
}

fn collect_single_value_constraints(
    expr: &Expression,
    constraints: &mut HashMap<u32, SingleValueConstraint>,
) -> bool {
    match expr {
        Expression::BinaryOp { left, op, right } => match op {
            BoundBinaryOp::And => {
                collect_single_value_constraints(left, constraints)
                    && collect_single_value_constraints(right, constraints)
            },
            BoundBinaryOp::Or => false,
            BoundBinaryOp::Eq => {
                if let Some(slot_id) = slot_id_equals_single_value(left, right) {
                    set_constraint(constraints, slot_id, SingleValueConstraint::EqLike);
                } else if let Some(slot_id) = slot_id_equals_single_value(right, left) {
                    set_constraint(constraints, slot_id, SingleValueConstraint::EqLike);
                }
                true
            },
            _ => true,
        },
        Expression::InList {
            expr,
            list,
            negated,
        } => {
            if !*negated && list.len() == 1 {
                if let Expression::SlotRef(slot_id) = expr.as_ref() {
                    set_constraint(constraints, *slot_id, SingleValueConstraint::EqLike);
                }
            }
            true
        },
        Expression::IsNull { expr, negated } => {
            if !*negated {
                if let Expression::SlotRef(slot_id) = expr.as_ref() {
                    set_constraint(constraints, *slot_id, SingleValueConstraint::IsNull);
                }
            }
            true
        },
        _ => true,
    }
}

fn slot_id_equals_single_value(left: &Expression, right: &Expression) -> Option<u32> {
    let Expression::SlotRef(slot_id) = left else {
        return None;
    };
    if is_single_value_expr(right) {
        Some(*slot_id)
    } else {
        None
    }
}

fn is_single_value_expr(expr: &Expression) -> bool {
    match expr {
        Expression::Literal(_) => true,
        Expression::Cast { expr, .. } => is_single_value_expr(expr),
        _ => false,
    }
}

fn set_constraint(
    constraints: &mut HashMap<u32, SingleValueConstraint>,
    slot_id: u32,
    constraint: SingleValueConstraint,
) {
    match constraints.get(&slot_id) {
        Some(SingleValueConstraint::EqLike) => {},
        Some(SingleValueConstraint::IsNull) if constraint == SingleValueConstraint::EqLike => {
            constraints.insert(slot_id, SingleValueConstraint::EqLike);
        },
        None => {
            constraints.insert(slot_id, constraint);
        },
        _ => {},
    }
}

fn key_satisfied(
    key: &ResolvedKey,
    constraints: &HashMap<u32, SingleValueConstraint>,
    input_columns: &[InferColumn],
) -> bool {
    if key.slot_ids.is_empty() {
        return false;
    }

    for slot_id in &key.slot_ids {
        let Some(constraint) = constraints.get(slot_id) else {
            return false;
        };
        if *constraint == SingleValueConstraint::EqLike {
            continue;
        }
        let Some(column) = input_columns
            .iter()
            .find(|column| column.slot_id.is_some_and(|value| value == *slot_id))
        else {
            return false;
        };
        if column.nullable {
            return false;
        }
    }

    true
}
