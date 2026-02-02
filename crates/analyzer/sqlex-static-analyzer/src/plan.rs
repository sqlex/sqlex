//! Query plan node definitions
//!
//! Defines the tree structure for representing SQL queries
//! in a form suitable for type and nullability inference.

use sqlex_common::DataType;
use sqlparser::ast::Expr;

/// Query plan node representing the logical structure of a SQL query
#[derive(Debug, Clone)]
pub enum PlanNode {
    // === Data Sources ===
    /// Table scan: FROM table [AS alias]
    TableScan {
        table: String,
        alias: Option<String>,
    },

    /// VALUES clause: VALUES (1, 'a'), (2, 'b')
    Values {
        rows: Vec<Vec<TypedExpr>>,
        column_names: Vec<String>,
    },

    /// Subquery as data source: (SELECT ...) AS alias
    Subquery { query: Box<PlanNode>, alias: String },

    // === Joins ===
    /// JOIN operation
    Join {
        kind: JoinKind,
        left: Box<PlanNode>,
        right: Box<PlanNode>,
        condition: Option<JoinCondition>,
    },

    /// LATERAL subquery (can reference columns from left side)
    LateralJoin {
        left: Box<PlanNode>,
        lateral: Box<PlanNode>,
        kind: JoinKind,
    },

    // === Filtering ===
    /// WHERE / HAVING filter
    Filter {
        input: Box<PlanNode>,
        predicate: Box<TypedExpr>,
    },

    // === Projection ===
    /// SELECT expr1 AS a, expr2 AS b
    Project {
        input: Box<PlanNode>,
        columns: Vec<ProjectColumn>,
    },

    /// SELECT DISTINCT
    Distinct { input: Box<PlanNode> },

    /// SELECT DISTINCT ON (expr) (PostgreSQL)
    DistinctOn {
        input: Box<PlanNode>,
        on_exprs: Vec<TypedExpr>,
    },

    // === Aggregation ===
    /// GROUP BY + aggregate functions
    Aggregate {
        input: Box<PlanNode>,
        group_by: Vec<TypedExpr>,
        aggregates: Vec<AggregateExpr>,
        grouping_mode: Option<GroupingMode>,
    },

    // === Window Functions ===
    /// Window function: expr OVER (PARTITION BY ... ORDER BY ...)
    Window {
        input: Box<PlanNode>,
        functions: Vec<WindowExpr>,
    },

    // === Ordering/Pagination ===
    /// ORDER BY
    Sort {
        input: Box<PlanNode>,
        order_by: Vec<OrderByExpr>,
    },

    /// LIMIT / OFFSET / FETCH
    Limit {
        input: Box<PlanNode>,
        limit: Option<u64>,
        offset: Option<u64>,
    },

    // === Set Operations ===
    /// UNION / INTERSECT / EXCEPT
    SetOperation {
        op: SetOp,
        all: bool,
        left: Box<PlanNode>,
        right: Box<PlanNode>,
    },

    // === CTE ===
    /// WITH cte AS (...) SELECT ...
    WithCTE {
        ctes: Vec<CTEDef>,
        body: Box<PlanNode>,
    },

    /// CTE reference (referencing a CTE in the body)
    CTERef { name: String, alias: Option<String> },
}

/// Join type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinKind {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

/// Join condition
#[derive(Debug, Clone)]
pub enum JoinCondition {
    /// ON expr
    On(Box<TypedExpr>),
    /// USING (col1, col2)
    Using(Vec<String>),
    /// NATURAL JOIN
    Natural,
}

/// Set operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOp {
    Union,
    Intersect,
    Except,
}

/// Grouping mode for advanced GROUP BY
#[derive(Debug, Clone)]
pub enum GroupingMode {
    GroupingSets(Vec<Vec<TypedExpr>>),
    Cube,
    Rollup,
}

/// Project column (SELECT item)
#[derive(Debug, Clone)]
pub struct ProjectColumn {
    pub alias: Option<String>,
    pub expr: TypedExpr,
}

/// CTE definition
#[derive(Debug, Clone)]
pub struct CTEDef {
    pub name: String,
    pub columns: Option<Vec<String>>,
    pub query: Box<PlanNode>,
    pub recursive: bool,
    pub materialized: Option<bool>,
}

/// Expression with inferred type information
#[derive(Debug, Clone)]
pub struct TypedExpr {
    pub expr: Expr,
    pub data_type: DataType,
    pub nullable: bool,
}

impl TypedExpr {
    /// Create a new typed expression
    pub fn new(expr: Expr, data_type: DataType, nullable: bool) -> Self {
        Self {
            expr,
            data_type,
            nullable,
        }
    }
}

/// Aggregate expression
#[derive(Debug, Clone)]
pub struct AggregateExpr {
    pub function: AggregateFunction,
    pub args: Vec<TypedExpr>,
    pub distinct: bool,
    pub filter: Option<Box<TypedExpr>>,
    pub order_by: Vec<OrderByExpr>,
}

/// Window expression
#[derive(Debug, Clone)]
pub struct WindowExpr {
    pub function: WindowFunction,
    pub args: Vec<TypedExpr>,
    pub partition_by: Vec<TypedExpr>,
    pub order_by: Vec<OrderByExpr>,
    pub frame: Option<WindowFrame>,
}

/// ORDER BY expression
#[derive(Debug, Clone)]
pub struct OrderByExpr {
    pub expr: TypedExpr,
    pub asc: bool,
    pub nulls_first: Option<bool>,
}

/// Window frame specification
#[derive(Debug, Clone)]
pub struct WindowFrame {
    pub units: WindowFrameUnits,
    pub start: WindowFrameBound,
    pub end: Option<WindowFrameBound>,
}

/// Window frame units
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowFrameUnits {
    Rows,
    Range,
    Groups,
}

/// Window frame bound
#[derive(Debug, Clone)]
pub enum WindowFrameBound {
    CurrentRow,
    Preceding(Option<u64>),
    Following(Option<u64>),
}

/// Aggregate function
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    ArrayAgg,
    StringAgg,
    JsonAgg,
    First,
    Last,
    Custom(String),
}

/// Window function
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowFunction {
    /// Aggregate function used as window function
    Aggregate(AggregateFunction),
    /// Dedicated window functions
    RowNumber,
    Rank,
    DenseRank,
    NTile,
    Lead,
    Lag,
    FirstValue,
    LastValue,
    NthValue,
    PercentRank,
    CumeDist,
}
