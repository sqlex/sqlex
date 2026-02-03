use sqlex_common::DataType;
use sqlparser::ast::Expr;

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

/// ORDER BY expression
#[derive(Debug, Clone)]
pub struct OrderByExpr {
    pub expr: TypedExpr,
    pub asc: bool,
    pub nulls_first: Option<bool>,
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
