use sqlex_common::types::DataType;

use crate::ir::{
    auxiliary::{BinaryOp, UnaryOp},
    relational::RelationalExpr,
};

/// Scalar expression — computes a single value.
/// Used in Selection conditions, Projection columns, Join conditions, etc.
#[derive(Debug, Clone)]
pub enum ScalarExpr {
    // ── Basics ──
    ColumnRef {
        table: Option<String>,
        column: String,
    },
    Literal(LiteralValue),

    // ── Operators ──
    BinaryOp {
        left: Box<ScalarExpr>,
        op: BinaryOp,
        right: Box<ScalarExpr>,
    },
    UnaryOp {
        op: UnaryOp,
        expr: Box<ScalarExpr>,
    },

    // ── Scalar functions ──
    Function {
        name: String,
        args: Vec<ScalarExpr>,
    },
    Cast {
        expr: Box<ScalarExpr>,
        target_type: DataType,
    },

    // ── Aggregate functions (appear inside Aggregation/Window nodes) ──
    AggregateCall {
        name: String,
        args: Vec<ScalarExpr>,
        distinct: bool,
    },

    // ── Window functions ──
    WindowCall {
        name: String,
        args: Vec<ScalarExpr>,
        partition_by: Vec<ScalarExpr>,
        order_by: Vec<crate::ir::auxiliary::SortKey>,
        is_aggregate_window: bool,
    },

    // ── Predicates ──
    IsNull {
        expr: Box<ScalarExpr>,
        negated: bool,
    },
    InList {
        expr: Box<ScalarExpr>,
        list: Vec<ScalarExpr>,
        negated: bool,
    },
    Between {
        expr: Box<ScalarExpr>,
        low: Box<ScalarExpr>,
        high: Box<ScalarExpr>,
        negated: bool,
    },

    // ── Conditional ──
    Case {
        operand: Option<Box<ScalarExpr>>,
        when_clauses: Vec<WhenClause>,
        else_result: Option<Box<ScalarExpr>>,
    },

    // ── Subqueries (embedded relational expressions) ──
    ScalarSubquery(Box<RelationalExpr>),
    InSubquery {
        expr: Box<ScalarExpr>,
        subquery: Box<RelationalExpr>,
        negated: bool,
    },
    Exists {
        subquery: Box<RelationalExpr>,
        negated: bool,
    },

    // ── Wildcard (for COUNT(*) and SELECT *) ──
    Wildcard,

    // ── Qualified wildcard (for SELECT t.*) ──
    QualifiedWildcard {
        table: String,
    },

    /// Placeholder for expressions that failed to algebraize.
    /// Diagnostics have already been reported; this allows analysis to continue.
    Error,
}

#[derive(Debug, Clone)]
pub enum LiteralValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
}

#[derive(Debug, Clone)]
pub struct WhenClause {
    pub condition: ScalarExpr,
    pub result: ScalarExpr,
}
