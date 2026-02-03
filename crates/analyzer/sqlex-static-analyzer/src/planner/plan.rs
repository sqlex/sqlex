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

use std::collections::HashMap;

use sqlex_common::ColumnInfo;

use crate::schema::Schema;

/// CTE columns context for resolving CTERef nodes
pub type CTEContext = HashMap<String, Vec<ColumnInfo>>;

impl PlanNode {
    /// Recursively derive the output columns of this plan node.
    ///
    /// This is the core inference method - each node type knows how to
    /// compute its output schema based on its children and the database schema.
    pub fn columns(&self, schema: &Schema) -> Vec<ColumnInfo> {
        self.columns_with_cte(schema, &HashMap::new())
    }

    /// Internal method with CTE context for resolving CTE references.
    pub fn columns_with_cte(&self, schema: &Schema, cte_ctx: &CTEContext) -> Vec<ColumnInfo> {
        match self {
            // === Data Sources ===
            PlanNode::TableScan { table, .. } => {
                // Get columns directly from schema
                schema
                    .get_table(table)
                    .map(|t| {
                        t.columns
                            .iter()
                            .map(|c| ColumnInfo {
                                name: c.name.clone(),
                                data_type: c.data_type.clone(),
                                nullability: c.nullable,
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            },

            PlanNode::Values { rows, column_names } => {
                // Infer from the first row, check nullability across all rows
                if let Some(first_row) = rows.first() {
                    first_row
                        .iter()
                        .enumerate()
                        .map(|(i, expr)| {
                            let name = column_names
                                .get(i)
                                .cloned()
                                .unwrap_or_else(|| format!("column{}", i + 1));
                            // Check if any row has NULL at this position
                            let nullable = rows
                                .iter()
                                .any(|r| r.get(i).map(|e| e.nullable).unwrap_or(true));
                            ColumnInfo {
                                name,
                                data_type: expr.data_type.clone(),
                                nullability: nullable,
                            }
                        })
                        .collect()
                } else {
                    vec![]
                }
            },

            PlanNode::Subquery { query, .. } => {
                // Subquery output is its inner query's output
                query.columns_with_cte(schema, cte_ctx)
            },

            PlanNode::CTERef { name, .. } => {
                // Look up CTE in context
                cte_ctx.get(name).cloned().unwrap_or_default()
            },

            // === Joins ===
            PlanNode::Join {
                kind, left, right, ..
            } => {
                let mut left_cols = left.columns_with_cte(schema, cte_ctx);
                let mut right_cols = right.columns_with_cte(schema, cte_ctx);

                // Apply nullability based on join type
                match kind {
                    JoinKind::Left => {
                        // Right side becomes nullable
                        for col in &mut right_cols {
                            col.nullability = true;
                        }
                    },
                    JoinKind::Right => {
                        // Left side becomes nullable
                        for col in &mut left_cols {
                            col.nullability = true;
                        }
                    },
                    JoinKind::Full => {
                        // Both sides become nullable
                        for col in &mut left_cols {
                            col.nullability = true;
                        }
                        for col in &mut right_cols {
                            col.nullability = true;
                        }
                    },
                    JoinKind::Inner | JoinKind::Cross => {
                        // No nullability changes
                    },
                }

                left_cols.extend(right_cols);
                left_cols
            },

            PlanNode::LateralJoin {
                left,
                lateral,
                kind,
            } => {
                let mut left_cols = left.columns_with_cte(schema, cte_ctx);
                let mut right_cols = lateral.columns_with_cte(schema, cte_ctx);

                if matches!(kind, JoinKind::Left) {
                    for col in &mut right_cols {
                        col.nullability = true;
                    }
                }

                left_cols.extend(right_cols);
                left_cols
            },

            // === Filtering ===
            PlanNode::Filter { input, .. } => {
                // Filter doesn't change columns
                input.columns_with_cte(schema, cte_ctx)
            },

            // === Projection ===
            PlanNode::Project { columns, .. } => {
                // Project defines new columns from expressions
                columns
                    .iter()
                    .enumerate()
                    .map(|(i, c)| ColumnInfo {
                        name: c.alias.clone().unwrap_or_else(|| format!("col_{}", i)),
                        data_type: c.expr.data_type.clone(),
                        nullability: c.expr.nullable,
                    })
                    .collect()
            },

            PlanNode::Distinct { input } => {
                // Distinct doesn't change columns
                input.columns_with_cte(schema, cte_ctx)
            },

            PlanNode::DistinctOn { input, .. } => {
                // Distinct ON doesn't change columns
                input.columns_with_cte(schema, cte_ctx)
            },

            // === Aggregation ===
            PlanNode::Aggregate {
                group_by,
                aggregates,
                ..
            } => {
                // Output: group_by columns + aggregate results
                let mut cols: Vec<ColumnInfo> = group_by
                    .iter()
                    .enumerate()
                    .map(|(i, expr)| ColumnInfo {
                        name: format!("group_{}", i),
                        data_type: expr.data_type.clone(),
                        nullability: expr.nullable,
                    })
                    .collect();

                for (i, agg) in aggregates.iter().enumerate() {
                    let (data_type, nullable) = aggregate_result_type(&agg.function, &agg.args);
                    cols.push(ColumnInfo {
                        name: format!("agg_{}", i),
                        data_type,
                        nullability: nullable,
                    });
                }

                cols
            },

            // === Window Functions ===
            PlanNode::Window { input, functions } => {
                // Window adds columns to input
                let mut cols = input.columns_with_cte(schema, cte_ctx);

                for (i, win) in functions.iter().enumerate() {
                    let (data_type, nullable) = window_result_type(&win.function, &win.args);
                    cols.push(ColumnInfo {
                        name: format!("window_{}", i),
                        data_type,
                        nullability: nullable,
                    });
                }

                cols
            },

            // === Ordering/Pagination ===
            PlanNode::Sort { input, .. } => {
                // Sort doesn't change columns
                input.columns_with_cte(schema, cte_ctx)
            },

            PlanNode::Limit { input, .. } => {
                // Limit doesn't change columns
                input.columns_with_cte(schema, cte_ctx)
            },

            // === Set Operations ===
            PlanNode::SetOperation { left, .. } => {
                // Use left side's columns (SQL standard: names come from left)
                left.columns_with_cte(schema, cte_ctx)
            },

            // === CTE ===
            PlanNode::WithCTE { ctes, body } => {
                // Build CTE context and resolve body
                let mut new_ctx = cte_ctx.clone();
                for cte in ctes {
                    let cte_cols = cte.query.columns_with_cte(schema, &new_ctx);
                    // Apply column aliases if specified
                    let final_cols = if let Some(ref aliases) = cte.columns {
                        cte_cols
                            .into_iter()
                            .zip(aliases.iter())
                            .map(|(mut c, alias)| {
                                c.name = alias.clone();
                                c
                            })
                            .collect()
                    } else {
                        cte_cols
                    };
                    new_ctx.insert(cte.name.clone(), final_cols);
                }
                body.columns_with_cte(schema, &new_ctx)
            },
        }
    }
}

/// Determine the result type and nullability of an aggregate function.
fn aggregate_result_type(func: &AggregateFunction, args: &[TypedExpr]) -> (DataType, bool) {
    let input_type = args
        .first()
        .map(|a| a.data_type.clone())
        .unwrap_or(DataType::Int);

    match func {
        AggregateFunction::Count => (DataType::BigInt, false), // COUNT never returns NULL
        AggregateFunction::Sum => (input_type, true),          // SUM can return NULL for empty set
        AggregateFunction::Avg => (DataType::Double, true),    // AVG can return NULL
        AggregateFunction::Min | AggregateFunction::Max => (input_type, true), // Can return NULL
        AggregateFunction::ArrayAgg => (DataType::Array(Box::new(input_type)), true),
        AggregateFunction::StringAgg => (DataType::Text, true),
        AggregateFunction::JsonAgg => (DataType::Json, true),
        AggregateFunction::First | AggregateFunction::Last => (input_type, true),
        AggregateFunction::Custom(_) => (input_type, true),
    }
}

/// Determine the result type and nullability of a window function.
fn window_result_type(func: &WindowFunction, args: &[TypedExpr]) -> (DataType, bool) {
    match func {
        WindowFunction::Aggregate(agg) => aggregate_result_type(agg, args),
        WindowFunction::RowNumber
        | WindowFunction::Rank
        | WindowFunction::DenseRank
        | WindowFunction::NTile => (DataType::BigInt, false),
        WindowFunction::Lead
        | WindowFunction::Lag
        | WindowFunction::FirstValue
        | WindowFunction::LastValue
        | WindowFunction::NthValue => {
            let input_type = args
                .first()
                .map(|a| a.data_type.clone())
                .unwrap_or(DataType::Int);
            (input_type, true) // These can return NULL
        },
        WindowFunction::PercentRank | WindowFunction::CumeDist => (DataType::Double, false),
    }
}
