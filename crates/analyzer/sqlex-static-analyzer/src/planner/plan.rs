//! Query plan node definitions
//!
//! Defines the tree structure for representing SQL queries
//! in a form suitable for type and nullability inference.

/// Query plan node representing the logical structure of a SQL query
use std::any::Any;
use std::{collections::HashMap, fmt::Debug};

use sqlex_common::DataType;
use sqlparser::ast::Expr;

/// CTE columns context for resolving CTERef nodes
/// Column information specific to the internal query plan
#[derive(Debug, Clone)]
pub struct PlanNodeColumn {
    pub name: String,
    pub data_type: DataType,
    pub nullability: bool,
    pub origin_table: Option<String>,
    pub origin_column: Option<String>,
}

pub type CTEContext = HashMap<String, Vec<PlanNodeColumn>>;

// ============================================================================
//  Traits & Macros
// ============================================================================

/// Core logic trait that concrete nodes must implement.
///
/// This trait defines the specific behavior of a logical operator,
/// such as how to infer its output schema.
pub trait LogicalNode: Debug + Clone + Send + Sync + 'static {
    /// Recursively derive the output columns of this plan node.
    fn columns(&self) -> &[PlanNodeColumn];
}

/// The main object-safe trait for query plan nodes.
///
/// This trait is automatically implemented for any type that implements `LogicalNode`.
/// It provides dynamic dispatch capabilities (`as_any`, `box_clone`) and
/// exposes the core logic methods.
pub trait PlanNode: Debug + Send + Sync + 'static {
    fn as_any(&self) -> &dyn Any;
    fn columns(&self) -> &[PlanNodeColumn];
    fn box_clone(&self) -> Box<dyn PlanNode>;
}

/// Blanket implementation: Any `LogicalNode` is a `PlanNode`.
impl<T> PlanNode for T
where
    T: LogicalNode,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn columns(&self) -> &[PlanNodeColumn] {
        self.columns()
    }

    fn box_clone(&self) -> Box<dyn PlanNode> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn PlanNode> {
    fn clone(&self) -> Box<dyn PlanNode> {
        self.box_clone()
    }
}

/// Helper macro to simplify downcasting `PlanNode` trait objects.
///
/// # Usage
/// ```ignore
/// match_plan!(node, {
///     scan: TableScanNode => { ... },
///     proj: ProjectNode => { ... },
///     _ => { ... }
/// })
/// ```
#[macro_export]
macro_rules! match_plan {
    ($node:expr, {
        $( $var:ident : $type:ty => $body:expr ),*,
        _ => $default:expr
    }) => {
        {
            let node_ref = $node.as_any();
            if false {
                unreachable!()
            }
            $(
                else if let Some($var) = node_ref.downcast_ref::<$type>() {
                    $body
                }
            )*
            else {
                $default
            }
        }
    };
}

// ============================================================================
//  Auxiliary Types (JoinKind, TypedExpr, etc.)
// ============================================================================

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
    pub query: Box<dyn PlanNode>,
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

// ============================================================================
//  Helper Functions
// ============================================================================

/// Determine the result type and nullability of an aggregate function.
pub fn aggregate_result_type(func: &AggregateFunction, args: &[TypedExpr]) -> (DataType, bool) {
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
pub fn window_result_type(func: &WindowFunction, args: &[TypedExpr]) -> (DataType, bool) {
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
