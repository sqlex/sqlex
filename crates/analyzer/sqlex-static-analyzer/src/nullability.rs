//! Nullability inference engine
//!
//! Implements the rules for inferring whether expressions
//! and result columns can be NULL.

use sqlparser::ast::Expr;

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

/// Infer nullability for an expression
pub fn infer_expr_nullability(expr: &Expr, _plan: &PlanNode, _schema: &Schema) -> bool {
    match expr {
        // Literals are never null
        Expr::Value(sqlparser::ast::Value::Number(_, _))
        | Expr::Value(sqlparser::ast::Value::SingleQuotedString(_))
        | Expr::Value(sqlparser::ast::Value::DoubleQuotedString(_))
        | Expr::Value(sqlparser::ast::Value::Boolean(_)) => false,

        // NULL literal is always null
        Expr::Value(sqlparser::ast::Value::Null) => true,

        // Column references: inherit from source
        Expr::Identifier(_) | Expr::CompoundIdentifier(_) => {
            todo!("lookup column nullability from scope")
        },

        // Binary operations: nullable if either operand is nullable
        Expr::BinaryOp {
            left: _, right: _, ..
        } => {
            todo!("infer nullability for binary operation: left.nullable || right.nullable")
        },

        // COALESCE: nullable only if ALL arguments are nullable
        Expr::Function(func) if is_coalesce(&func.name) => {
            todo!("COALESCE is nullable only if all args are nullable")
        },

        // NULLIF: always nullable
        Expr::Function(func) if is_nullif(&func.name) => true,

        // CASE: complex rules
        Expr::Case { .. } => {
            todo!("CASE nullability: no ELSE or any branch nullable")
        },

        // Subquery: always nullable (may return no rows)
        Expr::Subquery(_) => true,

        // Other expressions: default to nullable (conservative)
        _ => true,
    }
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

/// Determine nullability for JOIN columns
pub fn join_nullability(
    join_kind: JoinKind,
    _left: &PlanNode,
    _right: &PlanNode,
    _condition: &Option<crate::plan::JoinCondition>,
    _schema: &Schema,
) -> ColumnNullability {
    match join_kind {
        JoinKind::Inner | JoinKind::Cross => {
            // INNER/CROSS JOIN: preserve original nullability
            ColumnNullability::Preserve
        },
        JoinKind::Left => {
            // LEFT JOIN: check for FK guarantee
            if has_fk_guarantee_right_to_left(_left, _right, _condition, _schema) {
                ColumnNullability::Preserve
            } else {
                ColumnNullability::ForceNullable(JoinSide::Right)
            }
        },
        JoinKind::Right => {
            // RIGHT JOIN: symmetric to LEFT
            if has_fk_guarantee_left_to_right(_left, _right, _condition, _schema) {
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

/// Check if there's a FK from right table to left table that guarantees matches
fn has_fk_guarantee_right_to_left(
    _left: &PlanNode,
    _right: &PlanNode,
    _condition: &Option<crate::plan::JoinCondition>,
    _schema: &Schema,
) -> bool {
    todo!("check FK: right.join_col REFERENCES left.join_col AND right.join_col IS NOT NULL")
}

/// Check if there's a FK from left table to right table that guarantees matches
fn has_fk_guarantee_left_to_right(
    _left: &PlanNode,
    _right: &PlanNode,
    _condition: &Option<crate::plan::JoinCondition>,
    _schema: &Schema,
) -> bool {
    todo!("check FK: left.join_col REFERENCES right.join_col AND left.join_col IS NOT NULL")
}

/// Check if function name is COALESCE
fn is_coalesce(name: &sqlparser::ast::ObjectName) -> bool {
    name.0
        .last()
        .map(|n| n.value.to_uppercase() == "COALESCE")
        .unwrap_or(false)
}

/// Check if function name is NULLIF
fn is_nullif(name: &sqlparser::ast::ObjectName) -> bool {
    name.0
        .last()
        .map(|n| n.value.to_uppercase() == "NULLIF")
        .unwrap_or(false)
}
