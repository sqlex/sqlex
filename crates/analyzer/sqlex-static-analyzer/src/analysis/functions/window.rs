use sqlex_common::{dialect::Dialect, types::DataType};

use super::{aggregate::AggregateFunction, arity::FunctionArity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowFunction {
    Aggregate(AggregateFunction),
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
    pub(crate) fn from_name(name: &str) -> Option<Self> {
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

    pub(crate) fn arity(&self) -> FunctionArity {
        match self {
            Self::RowNumber | Self::Rank | Self::DenseRank | Self::PercentRank | Self::CumeDist => {
                FunctionArity::Exact(0)
            },
            Self::NTile => FunctionArity::Exact(1),
            Self::Lead | Self::Lag => FunctionArity::Between { min: 1, max: 3 },
            Self::FirstValue | Self::LastValue => FunctionArity::Exact(1),
            Self::NthValue => FunctionArity::Exact(2),
            Self::Aggregate(agg) => agg.arity(),
        }
    }

    pub(crate) fn infer_type(&self, dialect: Dialect, arg_types: &[DataType]) -> (DataType, bool) {
        match self {
            Self::RowNumber | Self::Rank | Self::DenseRank | Self::NTile => {
                let data_type = match dialect {
                    Dialect::MySQL => DataType::BigInt(true),
                    Dialect::Postgres | Dialect::SQLite => DataType::BigInt(false),
                };
                (data_type, false)
            },
            Self::PercentRank | Self::CumeDist => (DataType::Double, false),
            Self::Lead | Self::Lag | Self::FirstValue | Self::LastValue | Self::NthValue => {
                let data_type = arg_types.first().cloned().unwrap_or(DataType::Int(false));
                (data_type, true)
            },
            Self::Aggregate(agg) => agg.infer_type(dialect, arg_types),
        }
    }

    pub(crate) fn validate_argument_types(
        &self,
        dialect: Dialect,
        arg_types: &[DataType],
    ) -> Option<String> {
        match self {
            Self::Aggregate(agg) => agg.validate_argument_types(dialect, arg_types),
            _ => None,
        }
    }
}
