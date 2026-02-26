use std::collections::{HashMap, HashSet};

use crate::{
    algebraizer::model::{
        expression::{BoundBinaryOp, Expression},
        relation::{JoinKind, JoinNode, Relation, SelectionNode},
    },
    catalog::model::TableSchema,
    diagnostics::Diagnostic,
    infer::{
        Inferencer,
        model::{
            cardinality::CardInterval,
            metadata::{ColumnOrigin, InferColumn},
        },
        relation::predicate::{condition_implies_empty_result, selection_is_at_most_one},
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_selection_relation(
        &mut self,
        node: &SelectionNode,
    ) -> Result<crate::infer::model::metadata::InferMetadata, Diagnostic> {
        let mut child = self.infer_relation(&node.input)?;

        let _ = self.infer_expression(&node.condition, &child.columns)?;

        if condition_implies_empty_result(&node.condition, &child.keys, &child.columns) {
            child.cardinality = CardInterval::exactly_zero();
            return Ok(child);
        }

        if let Relation::Join(join_node) = node.input.as_ref() {
            child.cardinality = self.refine_join_cardinality_from_selection(
                child.cardinality,
                join_node,
                &node.condition,
            )?;

            if let Some(optimized_columns) =
                self.refine_join_nullability_from_selection(join_node, &node.condition)?
            {
                child.columns = optimized_columns;
            }
        }

        if selection_is_at_most_one(&node.condition, &child.keys, &child.columns) {
            child.cardinality = child.cardinality.constrain_at_most_one();
        }

        Ok(child)
    }

    /// Refine nullability of join result based on foreign key constraints.
    ///
    /// When a LEFT JOIN is performed on a NOT NULL foreign key column,
    /// and the join condition matches the FK definition pointing to a unique key,
    /// the referenced table's columns can preserve their original nullability
    /// instead of being forced to nullable.
    fn refine_join_nullability_from_selection(
        &mut self,
        join_node: &JoinNode,
        condition: &Expression,
    ) -> Result<Option<Vec<InferColumn>>, Diagnostic> {
        let (preserved_side, other_side, _preserved_is_left) = match join_node.kind {
            JoinKind::Left => (&join_node.left, &join_node.right, true),
            JoinKind::Right => (&join_node.right, &join_node.left, false),
            _ => return Ok(None),
        };

        let preserved_meta = self.infer_relation(preserved_side)?;
        let other_meta = self.infer_relation(other_side)?;

        let Some(join_pairs) =
            extract_equijoin_pairs(condition, &preserved_meta.columns, &other_meta.columns)
        else {
            return Ok(None);
        };

        if !self.join_guaranteed_by_fk(&join_pairs, &preserved_meta.columns, &other_meta.columns)? {
            return Ok(None);
        }

        let mut result_columns = preserved_meta.columns;
        result_columns.extend(other_meta.columns);

        Ok(Some(result_columns))
    }

    /// Check if the join pairs represent a valid FK→Unique constraint match.
    fn join_guaranteed_by_fk(
        &self,
        join_pairs: &[(u32, u32)],
        preserved_columns: &[InferColumn],
        other_columns: &[InferColumn],
    ) -> Result<bool, Diagnostic> {
        let mut preserved_col_map: HashMap<u32, &InferColumn> = HashMap::new();
        for col in preserved_columns {
            if let Some(slot_id) = col.slot_id {
                preserved_col_map.insert(slot_id, col);
            }
        }

        let mut other_col_map: HashMap<u32, &InferColumn> = HashMap::new();
        for col in other_columns {
            if let Some(slot_id) = col.slot_id {
                other_col_map.insert(slot_id, col);
            }
        }

        let mut fk_mapping: HashMap<String, String> = HashMap::new();
        let mut preserved_table_name: Option<String> = None;
        let mut other_table_name: Option<String> = None;

        for (preserved_slot, other_slot) in join_pairs {
            let preserved_col = preserved_col_map.get(preserved_slot).copied();
            let other_col = other_col_map.get(other_slot).copied();

            let (Some(preserved_col), Some(other_col)) = (preserved_col, other_col) else {
                return Ok(false);
            };

            if preserved_col.nullable {
                return Ok(false);
            }

            let (preserved_table, preserved_col_name) = match &preserved_col.origin {
                ColumnOrigin::Base { table, column } => (table.clone(), column.clone()),
                ColumnOrigin::Derived => return Ok(false),
            };

            let (other_table, other_col_name) = match &other_col.origin {
                ColumnOrigin::Base { table, column } => (table.clone(), column.clone()),
                ColumnOrigin::Derived => return Ok(false),
            };

            match &preserved_table_name {
                Some(name) if name != &preserved_table => return Ok(false),
                _ => preserved_table_name = Some(preserved_table),
            }
            match &other_table_name {
                Some(name) if name != &other_table => return Ok(false),
                _ => other_table_name = Some(other_table),
            }

            fk_mapping.insert(preserved_col_name, other_col_name);
        }

        let (Some(preserved_table), Some(other_table)) = (preserved_table_name, other_table_name)
        else {
            return Ok(false);
        };

        let Some(preserved_schema) = self.catalog.table(&preserved_table) else {
            return Ok(false);
        };
        let Some(other_schema) = self.catalog.table(&other_table) else {
            return Ok(false);
        };

        Ok(preserved_schema.foreign_keys.iter().any(|fk| {
            if fk.ref_table != other_table {
                return false;
            }

            if fk.columns.len() != fk_mapping.len() {
                return false;
            }

            if !fk_columns_are_not_null(preserved_schema, &fk.columns) {
                return false;
            }

            if !fk
                .columns
                .iter()
                .zip(fk.ref_columns.iter())
                .all(|(fk_col, ref_col)| {
                    fk_mapping
                        .get(fk_col)
                        .is_some_and(|mapped| mapped == ref_col)
                })
            {
                return false;
            }

            columns_cover_unique_key(other_schema, &fk.ref_columns)
        }))
    }
}

/// Extract equijoin pairs from a condition expression.
fn extract_equijoin_pairs(
    condition: &Expression,
    preserved_columns: &[InferColumn],
    other_columns: &[InferColumn],
) -> Option<Vec<(u32, u32)>> {
    let preserved_slots: HashSet<u32> =
        preserved_columns.iter().filter_map(|c| c.slot_id).collect();
    let other_slots: HashSet<u32> = other_columns.iter().filter_map(|c| c.slot_id).collect();

    if preserved_slots.is_empty() || other_slots.is_empty() {
        return None;
    }

    let mut pairs = HashSet::new();
    if !collect_equijoin_pairs(condition, &preserved_slots, &other_slots, &mut pairs) {
        return None;
    }

    if pairs.is_empty() {
        None
    } else {
        Some(pairs.into_iter().collect())
    }
}

/// Recursively collect equijoin pairs from expression.
fn collect_equijoin_pairs(
    expr: &Expression,
    preserved_slots: &HashSet<u32>,
    other_slots: &HashSet<u32>,
    pairs: &mut HashSet<(u32, u32)>,
) -> bool {
    match expr {
        Expression::BinaryOp { left, op, right } => match op {
            BoundBinaryOp::And => {
                collect_equijoin_pairs(left, preserved_slots, other_slots, pairs)
                    && collect_equijoin_pairs(right, preserved_slots, other_slots, pairs)
            },
            BoundBinaryOp::Eq => {
                let (Expression::SlotRef(left_slot), Expression::SlotRef(right_slot)) =
                    (left.as_ref(), right.as_ref())
                else {
                    return false;
                };

                if preserved_slots.contains(left_slot) && other_slots.contains(right_slot) {
                    pairs.insert((*left_slot, *right_slot));
                    true
                } else if preserved_slots.contains(right_slot) && other_slots.contains(left_slot) {
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

/// Check if all FK columns are NOT NULL.
fn fk_columns_are_not_null(table: &TableSchema, columns: &[String]) -> bool {
    columns.iter().all(|col_name| {
        table
            .columns
            .iter()
            .find(|c| &c.name == col_name)
            .is_some_and(|c| !c.nullable)
    })
}

/// Check if columns cover a unique key (primary key or unique constraint).
fn columns_cover_unique_key(table: &TableSchema, columns: &[String]) -> bool {
    if let Some(primary_key) = &table.primary_key {
        if same_column_set(&primary_key.columns, columns) {
            return true;
        }
    }

    table
        .unique_keys
        .iter()
        .any(|key| same_column_set(&key.columns, columns))
}

/// Check if two column sets are the same.
fn same_column_set(left: &[String], right: &[String]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let left_set: HashSet<&str> = left.iter().map(String::as_str).collect();
    let right_set: HashSet<&str> = right.iter().map(String::as_str).collect();

    left_set.len() == left.len() && right_set.len() == right.len() && left_set == right_set
}
