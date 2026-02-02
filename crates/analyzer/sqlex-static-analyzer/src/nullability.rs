//! Nullability inference engine
//!
//! Implements the rules for inferring whether expressions
//! and result columns can be NULL.

use crate::{
    plan::{AggregateFunction, JoinKind, PlanNode, TypedExpr, WindowFunction},
    schema::Schema,
};

/// Result of nullability analysis for JOIN columns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnNullability {
    /// Keep original nullability
    Preserve,
    /// Force columns to be nullable
    ForceNullable(JoinSide),
}

/// Which side of the JOIN to apply nullability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinSide {
    Left,
    Right,
    Both,
}

/// Infer nullability for binary operation
pub fn infer_binary_op_nullability(left_nullable: bool, right_nullable: bool) -> bool {
    left_nullable || right_nullable
}

/// Infer nullability for CASE expression
pub fn infer_case_nullability(
    has_else: bool,
    when_branches_nullable: &[bool],
    else_branch_nullable: Option<bool>,
) -> bool {
    // If no ELSE, implicitly NULL, so nullable
    if !has_else {
        return true;
    }

    // Check if any WHEN branch is nullable
    if when_branches_nullable.iter().any(|&n| n) {
        return true;
    }

    // Check if ELSE branch is nullable
    else_branch_nullable.unwrap_or(false)
}

/// Infer nullability for COALESCE
pub fn infer_coalesce_nullability(args_nullable: &[bool]) -> bool {
    // COALESCE is nullable only if ALL arguments are nullable
    args_nullable.iter().all(|&n| n)
}

/// Infer nullability for NULLIF
pub fn infer_nullif_nullability() -> bool {
    // NULLIF is always nullable because it returns NULL if args are equal
    true
}

/// Determine nullability for JOIN columns
pub fn join_nullability(
    join_kind: JoinKind,
    left: &PlanNode,
    right: &PlanNode,
    condition: &Option<crate::plan::JoinCondition>,
    schema: &Schema,
) -> ColumnNullability {
    match join_kind {
        JoinKind::Inner | JoinKind::Cross => {
            // INNER/CROSS JOIN: preserve original nullability
            ColumnNullability::Preserve
        },
        JoinKind::Left => {
            // LEFT JOIN: check for FK guarantee
            if check_fk_guarantee(JoinKind::Left, left, right, condition, schema) {
                ColumnNullability::Preserve
            } else {
                ColumnNullability::ForceNullable(JoinSide::Right)
            }
        },
        JoinKind::Right => {
            // RIGHT JOIN: symmetric to LEFT
            if check_fk_guarantee(JoinKind::Right, left, right, condition, schema) {
                ColumnNullability::Preserve
            } else {
                ColumnNullability::ForceNullable(JoinSide::Left)
            }
        },
        JoinKind::Full => {
            // FULL JOIN: both sides can be null
            ColumnNullability::ForceNullable(JoinSide::Both)
        },
    }
}

/// Extract table name from a PlanNode (if it's a TableScan)
fn extract_table_name_from_plan(plan: &PlanNode) -> Option<String> {
    match plan {
        PlanNode::TableScan { table, .. } => Some(table.clone()),
        PlanNode::Join { left, .. } => extract_table_name_from_plan(left),
        _ => None,
    }
}

/// Check FK guarantee using plan nodes
fn check_fk_guarantee(
    join_kind: JoinKind,
    left: &PlanNode,
    right: &PlanNode,
    condition: &Option<crate::plan::JoinCondition>,
    schema: &Schema,
) -> bool {
    let left_table = extract_table_name_from_plan(left);
    let right_table = extract_table_name_from_plan(right);

    match join_kind {
        JoinKind::Left => {
            // Check if LEFT table has FK to RIGHT table
            if let (Some(left_tbl), Some(right_tbl)) = (left_table, right_table) {
                if let Some(table_def) = schema.tables.get(&left_tbl) {
                    for fk in &table_def.foreign_keys {
                        if fk.ref_table == right_tbl {
                            // Check if FK columns are NOT NULL
                            let fk_cols_not_null = fk.columns.iter().all(|fk_col| {
                                table_def
                                    .columns
                                    .iter()
                                    .any(|col| col.name == *fk_col && !col.nullable)
                            });

                            if fk_cols_not_null
                                && matches!(condition, Some(crate::plan::JoinCondition::On(_)))
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        },
        JoinKind::Right => {
            // Check if RIGHT table has FK to LEFT table
            if let (Some(left_tbl), Some(right_tbl)) = (left_table, right_table) {
                if let Some(table_def) = schema.tables.get(&right_tbl) {
                    for fk in &table_def.foreign_keys {
                        if fk.ref_table == left_tbl {
                            let fk_cols_not_null = fk.columns.iter().all(|fk_col| {
                                table_def
                                    .columns
                                    .iter()
                                    .any(|col| col.name == *fk_col && !col.nullable)
                            });

                            if fk_cols_not_null
                                && matches!(condition, Some(crate::plan::JoinCondition::On(_)))
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        },
        _ => {},
    }
    false
}

/// Infer nullability for an aggregate function
pub fn infer_aggregate_nullability(func: &AggregateFunction, _args: &[TypedExpr]) -> bool {
    match func {
        // COUNT is never null (returns 0 for empty set)
        AggregateFunction::Count => false,

        // Other aggregates return NULL for empty set
        AggregateFunction::Sum
        | AggregateFunction::Avg
        | AggregateFunction::Min
        | AggregateFunction::Max
        | AggregateFunction::ArrayAgg
        | AggregateFunction::StringAgg
        | AggregateFunction::JsonAgg
        | AggregateFunction::First
        | AggregateFunction::Last
        | AggregateFunction::Custom(_) => true,
    }
}

/// Infer nullability for a window function
pub fn infer_window_nullability(func: &WindowFunction, args: &[TypedExpr]) -> bool {
    match func {
        // ROW_NUMBER, RANK, etc. are never null
        WindowFunction::RowNumber
        | WindowFunction::Rank
        | WindowFunction::DenseRank
        | WindowFunction::NTile
        | WindowFunction::PercentRank
        | WindowFunction::CumeDist => false,

        // LEAD/LAG can return NULL (beyond partition bounds)
        WindowFunction::Lead | WindowFunction::Lag => true,

        // FIRST_VALUE/LAST_VALUE/NTH_VALUE inherit from input
        WindowFunction::FirstValue | WindowFunction::LastValue | WindowFunction::NthValue => {
            args.first().map(|a| a.nullable).unwrap_or(true)
        },

        // Aggregate as window function
        WindowFunction::Aggregate(agg) => infer_aggregate_nullability(agg, args),
    }
}
