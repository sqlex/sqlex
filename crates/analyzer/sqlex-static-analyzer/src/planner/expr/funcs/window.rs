use sqlex_common::DataType;

use super::{
    super::{Expression, ExpressionNode, order_by::OrderByExpr},
    aggregate::AggregateFunction,
};

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

    /// Infer type for the new Expression system (takes DataType slices)
    pub fn infer_type(&self, arg_types: &[DataType]) -> (DataType, bool) {
        match self {
            WindowFunction::Aggregate(agg) => agg.infer_type(arg_types, &[]),
            WindowFunction::RowNumber
            | WindowFunction::Rank
            | WindowFunction::DenseRank
            | WindowFunction::NTile => (DataType::BigInt, false),
            WindowFunction::Lead
            | WindowFunction::Lag
            | WindowFunction::FirstValue
            | WindowFunction::LastValue
            | WindowFunction::NthValue => {
                let input_type = arg_types.first().cloned().unwrap_or(DataType::Int);
                (input_type, true)
            },
            WindowFunction::PercentRank | WindowFunction::CumeDist => (DataType::Double, false),
        }
    }
}

/// Window function expression
#[derive(Debug, Clone)]
pub struct WindowFunctionExpr {
    pub function: WindowFunction,
    pub args: Vec<Box<dyn Expression>>,
    pub partition_by: Vec<Box<dyn Expression>>,
    pub order_by: Vec<OrderByExpr>,
    pub frame: Option<WindowFrame>,
    pub return_type: DataType,
    pub is_nullable: bool,
}

impl ExpressionNode for WindowFunctionExpr {
    fn data_type(&self) -> DataType {
        self.return_type.clone()
    }

    fn nullable(&self) -> bool {
        self.is_nullable
    }
}

impl WindowFunctionExpr {
    /// Build a window function expression
    pub fn build(
        function: WindowFunction,
        args: Vec<Box<dyn Expression>>,
        partition_by: Vec<Box<dyn Expression>>,
        order_by: Vec<OrderByExpr>,
        frame: Option<WindowFrame>,
    ) -> Box<dyn Expression> {
        // Infer return type using the window function's logic
        let arg_types: Vec<DataType> = args.iter().map(|e| e.data_type()).collect();
        let (return_type, is_nullable) = function.infer_type(&arg_types);

        Box::new(WindowFunctionExpr {
            function,
            args,
            partition_by,
            order_by,
            frame,
            return_type,
            is_nullable,
        })
    }
}
