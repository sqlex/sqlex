use sqlex_analyzer::extension::DataTypeExt;
use sqlex_common::{dialect::Dialect, types::DataType};

#[derive(Debug, Clone)]
pub(crate) struct FunctionMeta {
    pub(crate) kind: FunctionKind,
    pub(crate) accepts_distinct: bool,
    pub(crate) allows_over: bool,
    pub(crate) requires_over: bool,
    pub(crate) arity: FunctionArity,
}

#[derive(Debug, Clone)]
pub(crate) enum FunctionKind {
    Scalar(ScalarFunction),
    Aggregate(AggregateFunction),
    Window(WindowFunction),
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum FunctionArity {
    Any,
    Exact(usize),
    AtLeast(usize),
    Between { min: usize, max: usize },
}

impl FunctionArity {
    pub(crate) fn matches(self, count: usize) -> bool {
        match self {
            Self::Any => true,
            Self::Exact(expected) => count == expected,
            Self::AtLeast(min) => count >= min,
            Self::Between { min, max } => count >= min && count <= max,
        }
    }

    pub(crate) fn describe(self) -> String {
        match self {
            Self::Any => "any number of".to_string(),
            Self::Exact(expected) => format!("{expected}"),
            Self::AtLeast(min) => format!("at least {min}"),
            Self::Between { min, max } => format!("between {min} and {max}"),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScalarFunction {
    // String
    Concat,
    ConcatWs,
    Upper,
    Lower,
    Trim,
    Ltrim,
    Rtrim,
    Substring,
    Replace,
    Left,
    Right,
    Repeat,
    Length,
    CharLength,
    OctetLength,
    BitLength,
    Position,

    // Numeric
    Abs,
    Ceil,
    Floor,
    Round,
    Truncate,
    Sqrt,
    Exp,
    Log,
    Ln,
    Log10,
    Log2,
    Power,
    Mod,
    Random,
    Sign,

    // Date/Time
    CurrentTimestamp,
    CurrentDate,
    CurrentTime,
    Date,
    Time,
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
    Extract,

    // Type Conversion
    Cast,
    Convert,

    // JSON
    JsonObject,
    JsonArray,
    ToJson,
    ToJsonb,

    // Control Flow / Boolean
    Coalesce,
    Nullif,
    Ifnull,
    Nvl,

    Custom(String),
}

impl ScalarFunction {
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name.to_uppercase().as_str() {
            // String
            "CONCAT" => Some(Self::Concat),
            "CONCAT_WS" => Some(Self::ConcatWs),
            "UPPER" => Some(Self::Upper),
            "LOWER" => Some(Self::Lower),
            "TRIM" => Some(Self::Trim),
            "LTRIM" => Some(Self::Ltrim),
            "RTRIM" => Some(Self::Rtrim),
            "SUBSTRING" | "SUBSTR" => Some(Self::Substring),
            "REPLACE" => Some(Self::Replace),
            "LEFT" => Some(Self::Left),
            "RIGHT" => Some(Self::Right),
            "REPEAT" => Some(Self::Repeat),
            "LENGTH" => Some(Self::Length),
            "CHAR_LENGTH" | "CHARACTER_LENGTH" => Some(Self::CharLength),
            "OCTET_LENGTH" => Some(Self::OctetLength),
            "BIT_LENGTH" => Some(Self::BitLength),
            "POSITION" | "STRPOS" => Some(Self::Position),

            // Numeric
            "ABS" => Some(Self::Abs),
            "CEIL" | "CEILING" => Some(Self::Ceil),
            "FLOOR" => Some(Self::Floor),
            "ROUND" => Some(Self::Round),
            "TRUNCATE" | "TRUNC" => Some(Self::Truncate),
            "SQRT" => Some(Self::Sqrt),
            "EXP" => Some(Self::Exp),
            "LOG" => Some(Self::Log),
            "LN" => Some(Self::Ln),
            "LOG10" => Some(Self::Log10),
            "LOG2" => Some(Self::Log2),
            "POWER" | "POW" => Some(Self::Power),
            "MOD" => Some(Self::Mod),
            "RANDOM" | "RAND" => Some(Self::Random),
            "SIGN" => Some(Self::Sign),

            // Date/Time
            "NOW" | "CURRENT_TIMESTAMP" => Some(Self::CurrentTimestamp),
            "CURRENT_DATE" => Some(Self::CurrentDate),
            "CURRENT_TIME" => Some(Self::CurrentTime),
            "DATE" => Some(Self::Date),
            "TIME" => Some(Self::Time),
            "YEAR" => Some(Self::Year),
            "MONTH" => Some(Self::Month),
            "DAY" => Some(Self::Day),
            "HOUR" => Some(Self::Hour),
            "MINUTE" => Some(Self::Minute),
            "SECOND" => Some(Self::Second),
            "EXTRACT" => Some(Self::Extract),

            // Type Conversion
            "CAST" => Some(Self::Cast),
            "CONVERT" => Some(Self::Convert),

            // JSON
            "JSON_OBJECT" => Some(Self::JsonObject),
            "JSON_ARRAY" => Some(Self::JsonArray),
            "TO_JSON" => Some(Self::ToJson),
            "TO_JSONB" => Some(Self::ToJsonb),

            // Control
            "COALESCE" => Some(Self::Coalesce),
            "NULLIF" => Some(Self::Nullif),
            "IFNULL" => Some(Self::Ifnull),
            "NVL" => Some(Self::Nvl),

            _ => None,
        }
    }

    pub(crate) fn infer_type(
        &self,
        dialect: Dialect,
        arg_types: &[DataType],
        arg_nullables: &[bool],
    ) -> (DataType, bool) {
        let input_type = arg_types.first().cloned().unwrap_or(DataType::Text);

        match self {
            Self::Concat
            | Self::ConcatWs
            | Self::Upper
            | Self::Lower
            | Self::Trim
            | Self::Ltrim
            | Self::Rtrim
            | Self::Substring
            | Self::Replace
            | Self::Left
            | Self::Right
            | Self::Repeat => (DataType::Text, true),

            Self::Length
            | Self::CharLength
            | Self::OctetLength
            | Self::BitLength
            | Self::Position => (DataType::Int(false), true),

            Self::Abs | Self::Ceil | Self::Floor | Self::Round | Self::Truncate | Self::Mod => {
                (input_type, true)
            },

            Self::Sqrt
            | Self::Exp
            | Self::Log
            | Self::Ln
            | Self::Log10
            | Self::Log2
            | Self::Power
            | Self::Random => (DataType::Double, true),

            Self::Sign => (DataType::Int(false), true),

            Self::CurrentTimestamp => (DataType::Timestamp, false),
            Self::CurrentDate => (DataType::Date, false),
            Self::CurrentTime => (DataType::Time, false),
            Self::Date => (DataType::Date, true),
            Self::Time => (DataType::Time, true),
            Self::Year
            | Self::Month
            | Self::Day
            | Self::Hour
            | Self::Minute
            | Self::Second
            | Self::Extract => (DataType::Int(false), true),

            Self::JsonObject | Self::JsonArray | Self::ToJson | Self::ToJsonb => {
                (DataType::Json, true)
            },

            Self::Coalesce | Self::Ifnull | Self::Nvl => {
                let is_nullable = arg_nullables.iter().all(|&n| n);
                let data_type = DataType::common_type(dialect, arg_types)
                    .unwrap_or_else(|| DataType::Custom("unknown".to_string()));
                (data_type, is_nullable)
            },
            Self::Nullif => (input_type, true),

            Self::Cast | Self::Convert => (input_type, true),

            Self::Custom(_) => (DataType::Custom("unknown".to_string()), true),
        }
    }

    pub(crate) fn arity(&self) -> FunctionArity {
        match self {
            Self::Concat | Self::ConcatWs => FunctionArity::AtLeast(2),
            Self::Upper
            | Self::Lower
            | Self::Trim
            | Self::Ltrim
            | Self::Rtrim
            | Self::Length
            | Self::CharLength
            | Self::OctetLength
            | Self::BitLength
            | Self::Abs
            | Self::Ceil
            | Self::Floor
            | Self::Sqrt
            | Self::Exp
            | Self::Ln
            | Self::Log10
            | Self::Log2
            | Self::Sign
            | Self::Date
            | Self::Time
            | Self::Year
            | Self::Month
            | Self::Day
            | Self::Hour
            | Self::Minute
            | Self::Second
            | Self::ToJson
            | Self::ToJsonb => FunctionArity::Exact(1),
            Self::Substring => FunctionArity::Between { min: 2, max: 3 },
            Self::Replace
            | Self::Left
            | Self::Right
            | Self::Repeat
            | Self::Position
            | Self::Power
            | Self::Mod
            | Self::Nullif
            | Self::Ifnull
            | Self::Nvl => FunctionArity::Exact(2),
            Self::Round | Self::Truncate | Self::Log => FunctionArity::Between { min: 1, max: 2 },
            Self::Random => FunctionArity::Between { min: 0, max: 1 },
            Self::CurrentTimestamp | Self::CurrentDate | Self::CurrentTime => {
                FunctionArity::Exact(0)
            },
            Self::Extract
            | Self::Cast
            | Self::Convert
            | Self::JsonObject
            | Self::JsonArray
            | Self::Custom(_) => FunctionArity::Any,
            Self::Coalesce => FunctionArity::AtLeast(1),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AggregateFunction {
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

    pub(crate) fn infer_type(&self, dialect: Dialect, arg_types: &[DataType]) -> (DataType, bool) {
        let input_type = arg_types.first().cloned().unwrap_or(DataType::Int(false));

        match self {
            Self::Count => (DataType::BigInt(false), false),
            Self::Sum => {
                let data_type = match input_type {
                    DataType::TinyInt(true)
                    | DataType::SmallInt(true)
                    | DataType::Int(true)
                    | DataType::BigInt(true) => DataType::BigInt(true),
                    DataType::TinyInt(false)
                    | DataType::SmallInt(false)
                    | DataType::Int(false)
                    | DataType::BigInt(false) => DataType::BigInt(false),
                    DataType::Decimal => match dialect {
                        Dialect::MySQL | Dialect::Postgres => DataType::Decimal,
                        Dialect::SQLite => DataType::Double,
                    },
                    DataType::Float | DataType::Double => DataType::Double,
                    _ => input_type,
                };
                (data_type, true)
            },
            Self::Avg => {
                let data_type = match dialect {
                    Dialect::SQLite => DataType::Double,
                    Dialect::MySQL | Dialect::Postgres => match input_type {
                        DataType::TinyInt(_)
                        | DataType::SmallInt(_)
                        | DataType::Int(_)
                        | DataType::BigInt(_)
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WindowFunction {
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
}

pub(crate) fn resolve_function(name: &str) -> FunctionMeta {
    let upper = name.to_uppercase();

    if let Some(window) = WindowFunction::from_name(&upper) {
        let arity = window.arity();
        return FunctionMeta {
            kind: FunctionKind::Window(window),
            accepts_distinct: false,
            allows_over: true,
            requires_over: true,
            arity,
        };
    }

    if let Some(agg) = AggregateFunction::from_name(&upper) {
        let arity = agg.arity();
        return FunctionMeta {
            kind: FunctionKind::Aggregate(agg),
            accepts_distinct: true,
            allows_over: true,
            requires_over: false,
            arity,
        };
    }

    if let Some(scalar) = ScalarFunction::from_name(&upper) {
        let arity = scalar.arity();
        return FunctionMeta {
            kind: FunctionKind::Scalar(scalar),
            accepts_distinct: false,
            allows_over: false,
            requires_over: false,
            arity,
        };
    }

    FunctionMeta {
        kind: FunctionKind::Unknown,
        accepts_distinct: false,
        allows_over: false,
        requires_over: false,
        arity: FunctionArity::Any,
    }
}
