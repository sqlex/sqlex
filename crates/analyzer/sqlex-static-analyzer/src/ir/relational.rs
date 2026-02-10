use crate::ir::{
    auxiliary::{
        AggregateColumn, JoinCondition, JoinKind, ProjectionColumn, SetOp, SortKey, WindowColumn,
    },
    scalar::ScalarExpr,
};

/// A relational algebra expression tree.
/// Each variant represents one relational operator.
/// The tree is built bottom-up during the Algebraize phase.
#[derive(Debug, Clone)]
pub enum RelationalExpr {
    // ── Leaf nodes (no input relation) ──
    Scan {
        table: String,
        alias: Option<String>,
    },
    Values {
        rows: Vec<Vec<ScalarExpr>>,
    },

    // ── Unary operators (one input relation) ──
    Selection {
        input: Box<RelationalExpr>,
        condition: ScalarExpr,
    },
    Projection {
        input: Box<RelationalExpr>,
        columns: Vec<ProjectionColumn>,
    },
    Aggregation {
        input: Box<RelationalExpr>,
        group_by: Vec<ScalarExpr>,
        aggregates: Vec<AggregateColumn>,
    },
    Window {
        input: Box<RelationalExpr>,
        window_exprs: Vec<WindowColumn>,
    },
    Distinct {
        input: Box<RelationalExpr>,
    },
    Sort {
        input: Box<RelationalExpr>,
        keys: Vec<SortKey>,
    },
    Limit {
        input: Box<RelationalExpr>,
        count: Option<ScalarExpr>,
        offset: Option<ScalarExpr>,
    },

    // ── Alias (derived table / CTE with a table alias) ──
    Alias {
        input: Box<RelationalExpr>,
        name: String,
    },

    // ── Binary operators (two input relations) ──
    Join {
        left: Box<RelationalExpr>,
        right: Box<RelationalExpr>,
        kind: JoinKind,
        condition: Option<JoinCondition>,
    },
    SetOperation {
        left: Box<RelationalExpr>,
        right: Box<RelationalExpr>,
        op: SetOp,
        all: bool,
    },
}
