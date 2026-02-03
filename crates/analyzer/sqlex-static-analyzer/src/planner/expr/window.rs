use sqlex_common::DataType;

use super::{aggregate::AggregateFunction, order_by::OrderByExpr, typed::TypedExpr};

/// Window expression
#[derive(Debug, Clone)]
pub struct WindowExpr {
    pub function: WindowFunction,
    pub args: Vec<TypedExpr>,
    pub partition_by: Vec<TypedExpr>,
    pub order_by: Vec<OrderByExpr>,
    pub frame: Option<WindowFrame>,
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

impl WindowFunction {
    pub fn from_name(name: &str) -> Option<Self> {
        let name_upper = name.to_uppercase();
        match name_upper.as_str() {
            "ROW_NUMBER" => Some(Self::RowNumber),
            "RANK" => Some(Self::Rank),
            "DENSE_RANK" => Some(Self::DenseRank),
            "NTILE" => Some(Self::NTile),
            "LEAD" => Some(Self::Lead),
            "LAG" => Some(Self::Lag),
            "FIRST_VALUE" => Some(Self::FirstValue),
            "LAST_VALUE" => Some(Self::LastValue),
            "NTH_VALUE" => Some(Self::NthValue),
            "PERCENT_RANK" => Some(Self::PercentRank),
            "CUME_DIST" => Some(Self::CumeDist),
            _ => None,
        }
    }

    /// Determine the result type and nullability of a window function.
    pub fn result_type(&self, args: &[TypedExpr]) -> (DataType, bool) {
        match self {
            WindowFunction::Aggregate(agg) => agg.result_type(args),
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
}
