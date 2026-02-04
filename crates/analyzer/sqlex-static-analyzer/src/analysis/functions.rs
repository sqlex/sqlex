use sqlex_common::DataType;

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
    Substr,
    Replace,
    Left,
    Right,
    Repeat,
    Length,
    CharLength,
    CharacterLength,
    OctetLength,
    BitLength,
    Position,
    Strpos,

    // Numeric
    Abs,
    Ceil,
    Ceiling,
    Floor,
    Round,
    Truncate,
    Trunc,
    Sqrt,
    Exp,
    Log,
    Ln,
    Log10,
    Log2,
    Power,
    Pow,
    Mod,
    Random,
    Rand,
    Sign,

    // Date/Time
    Now,
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

    // Fallback / Unknown
    Unknown,
    Custom(String),
}

#[derive(Debug, Clone)]
pub(crate) struct FunctionMeta {
    pub(crate) kind: FunctionKind,
    pub(crate) accepts_distinct: bool,
    pub(crate) allows_over: bool,
    pub(crate) requires_over: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum FunctionKind {
    Scalar(ScalarFunction),
    Aggregate(AggregateFunctionName),
    Window(WindowFunctionName),
    Unknown,
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
            "SUBSTRING" => Some(Self::Substring),
            "SUBSTR" => Some(Self::Substr),
            "REPLACE" => Some(Self::Replace),
            "LEFT" => Some(Self::Left),
            "RIGHT" => Some(Self::Right),
            "REPEAT" => Some(Self::Repeat),
            "LENGTH" => Some(Self::Length),
            "CHAR_LENGTH" => Some(Self::CharLength),
            "CHARACTER_LENGTH" => Some(Self::CharacterLength),
            "OCTET_LENGTH" => Some(Self::OctetLength),
            "BIT_LENGTH" => Some(Self::BitLength),
            "POSITION" => Some(Self::Position),
            "STRPOS" => Some(Self::Strpos),

            // Numeric
            "ABS" => Some(Self::Abs),
            "CEIL" => Some(Self::Ceil),
            "CEILING" => Some(Self::Ceiling),
            "FLOOR" => Some(Self::Floor),
            "TRUNCATE" => Some(Self::Truncate),
            "TRUNC" => Some(Self::Trunc),
            "SQRT" => Some(Self::Sqrt),
            "EXP" => Some(Self::Exp),
            "LOG" => Some(Self::Log),
            "LN" => Some(Self::Ln),
            "LOG10" => Some(Self::Log10),
            "LOG2" => Some(Self::Log2),
            "POWER" => Some(Self::Power),
            "POW" => Some(Self::Pow),
            "MOD" => Some(Self::Mod),
            "RANDOM" | "RAND" => Some(Self::Random),
            "SIGN" => Some(Self::Sign),

            // Date/Time
            "NOW" | "CURRENT_TIMESTAMP" => Some(Self::Now),
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

            // Handling functions that might map to multiple variants or direct parsing
            "ROUND" => Some(Self::Round),

            _ => None,
        }
    }

    pub(crate) fn infer_type(
        &self,
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
            | Self::Substr
            | Self::Replace
            | Self::Left
            | Self::Right
            | Self::Repeat => (DataType::Text, true),

            Self::Length
            | Self::CharLength
            | Self::CharacterLength
            | Self::OctetLength
            | Self::BitLength
            | Self::Position
            | Self::Strpos => (DataType::Int, true),

            Self::Abs
            | Self::Ceil
            | Self::Ceiling
            | Self::Floor
            | Self::Round
            | Self::Truncate
            | Self::Trunc
            | Self::Mod => (input_type, true),

            Self::Sqrt
            | Self::Exp
            | Self::Log
            | Self::Ln
            | Self::Log10
            | Self::Log2
            | Self::Power
            | Self::Pow
            | Self::Random
            | Self::Rand => (DataType::Double, true),

            Self::Sign => (DataType::Int, true),

            Self::Now | Self::CurrentTimestamp => (DataType::Timestamp, false),
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
            | Self::Extract => (DataType::Int, true),

            Self::JsonObject | Self::JsonArray | Self::ToJson | Self::ToJsonb => {
                (DataType::Json, true)
            },

            Self::Coalesce | Self::Ifnull | Self::Nvl => {
                let is_nullable = arg_nullables.iter().all(|&n| n);
                (input_type, is_nullable)
            },
            Self::Nullif => (input_type, true),

            Self::Cast | Self::Convert => (input_type, true),

            Self::Custom(_) | Self::Unknown => (DataType::Custom("unknown".to_string()), true),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AggregateFunctionName {
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

impl AggregateFunctionName {
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WindowFunctionName {
    Aggregate(AggregateFunctionName),
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

impl WindowFunctionName {
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
}

pub(crate) fn resolve_function(name: &str) -> FunctionMeta {
    let upper = name.to_uppercase();

    if let Some(window) = WindowFunctionName::from_name(&upper) {
        return FunctionMeta {
            kind: FunctionKind::Window(window),
            accepts_distinct: false,
            allows_over: true,
            requires_over: true,
        };
    }

    if let Some(agg) = AggregateFunctionName::from_name(&upper) {
        return FunctionMeta {
            kind: FunctionKind::Aggregate(agg),
            accepts_distinct: true,
            allows_over: true,
            requires_over: false,
        };
    }

    if let Some(scalar) = ScalarFunction::from_name(&upper) {
        return FunctionMeta {
            kind: FunctionKind::Scalar(scalar),
            accepts_distinct: false,
            allows_over: false,
            requires_over: false,
        };
    }

    FunctionMeta {
        kind: FunctionKind::Unknown,
        accepts_distinct: false,
        allows_over: false,
        requires_over: false,
    }
}
