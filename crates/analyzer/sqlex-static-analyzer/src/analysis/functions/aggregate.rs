use sqlex_analyzer::extension::DataTypeExt;
use sqlex_common::{dialect::Dialect, types::DataType};

use super::arity::FunctionArity;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    First,
    Last,
    ArrayAgg,
    JsonArrayAgg,
    JsonObjectAgg,
    StringAgg,
    Custom(String),
}

impl AggregateFunction {
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name.to_uppercase().as_str() {
            "COUNT" => Some(Self::Count),
            "SUM" => Some(Self::Sum),
            "AVG" => Some(Self::Avg),
            "MIN" => Some(Self::Min),
            "MAX" => Some(Self::Max),
            "FIRST" => Some(Self::First),
            "LAST" => Some(Self::Last),
            "ARRAY_AGG" => Some(Self::ArrayAgg),
            "JSON_AGG" | "JSON_ARRAYAGG" => Some(Self::JsonArrayAgg),
            "JSON_OBJECT_AGG" | "JSON_OBJECTAGG" => Some(Self::JsonObjectAgg),
            "STRING_AGG" | "GROUP_CONCAT" | "LISTAGG" => Some(Self::StringAgg),
            _ => None,
        }
    }

    pub(crate) fn arity(&self) -> FunctionArity {
        match self {
            Self::Count => FunctionArity::Between { min: 0, max: 1 },
            Self::StringAgg => FunctionArity::Exact(2),
            Self::JsonObjectAgg => FunctionArity::Exact(2),
            Self::Sum
            | Self::Avg
            | Self::Min
            | Self::Max
            | Self::First
            | Self::Last
            | Self::ArrayAgg
            | Self::JsonArrayAgg => FunctionArity::Exact(1),
            Self::Custom(_) => FunctionArity::Any,
        }
    }

    pub(crate) fn infer_type(&self, dialect: Dialect, arg_types: &[DataType]) -> (DataType, bool) {
        let input_type = arg_types.first().cloned().unwrap_or(DataType::Int);

        match self {
            Self::Count => (DataType::BigInt, false),
            Self::Sum => {
                let data_type = match input_type {
                    DataType::UnsignedTinyInt
                    | DataType::UnsignedSmallInt
                    | DataType::UnsignedInt
                    | DataType::UnsignedBigInt => DataType::UnsignedBigInt,
                    DataType::TinyInt | DataType::SmallInt | DataType::Int | DataType::BigInt => {
                        DataType::BigInt
                    },
                    DataType::Decimal => match dialect {
                        Dialect::MySQL | Dialect::Postgres => DataType::Decimal,
                        Dialect::SQLite => DataType::Double,
                    },
                    DataType::Float | DataType::Double => DataType::Double,
                    // For non-numeric types, MySQL and SQLite return Double
                    _ => match dialect {
                        Dialect::MySQL | Dialect::SQLite => DataType::Double,
                        Dialect::Postgres => input_type,
                    },
                };
                (data_type, true)
            },
            Self::Avg => {
                let data_type = match dialect {
                    Dialect::SQLite => DataType::Double,
                    Dialect::MySQL | Dialect::Postgres => match input_type {
                        DataType::TinyInt
                        | DataType::UnsignedTinyInt
                        | DataType::SmallInt
                        | DataType::UnsignedSmallInt
                        | DataType::Int
                        | DataType::UnsignedInt
                        | DataType::BigInt
                        | DataType::UnsignedBigInt
                        | DataType::Decimal => DataType::Decimal,
                        DataType::Float | DataType::Double => DataType::Double,
                        _ => DataType::Double,
                    },
                };
                (data_type, true)
            },
            Self::Min | Self::Max => (input_type, true),
            Self::First | Self::Last => (input_type, true),
            Self::ArrayAgg => (DataType::Array(Box::new(input_type)), true),
            Self::JsonArrayAgg | Self::JsonObjectAgg => (DataType::Json, true),
            Self::StringAgg => (DataType::Text, true),
            Self::Custom(_) => (input_type, true),
        }
    }

    pub(crate) fn validate_argument_types(
        &self,
        dialect: Dialect,
        arg_types: &[DataType],
    ) -> Option<String> {
        match self {
            Self::Sum | Self::Avg => {
                if dialect == Dialect::Postgres {
                    if let Some(first_arg) = arg_types.first() {
                        if !first_arg.is_numeric() {
                            return Some(format!("function {:?}(text) does not exist", self));
                        }
                    }
                }
                None
            },
            _ => None,
        }
    }
}
