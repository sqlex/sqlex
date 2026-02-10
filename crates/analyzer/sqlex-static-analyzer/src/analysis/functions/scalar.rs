use sqlex_analyzer::extension::DataTypeExt;
use sqlex_common::{dialect::Dialect, types::DataType};

use super::arity::FunctionArity;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalarFunction {
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

    pub(crate) fn infer_type(
        &self,
        dialect: Dialect,
        arg_types: &[DataType],
        arg_nullables: &[bool],
    ) -> (DataType, bool) {
        let input_type = arg_types.first().cloned().unwrap_or(DataType::Text);

        match self {
            Self::Concat | Self::ConcatWs => (DataType::Text, true),

            // String functions that preserve input type and nullability
            Self::Upper | Self::Lower | Self::Trim | Self::Ltrim | Self::Rtrim => {
                let nullable = arg_nullables.first().copied().unwrap_or(true);
                // MySQL and SQLite auto-convert non-text types to text
                if matches!(dialect, Dialect::MySQL | Dialect::SQLite) && !input_type.is_text_like()
                {
                    // MySQL returns VARCHAR for string functions
                    let text_type = if dialect == Dialect::MySQL {
                        DataType::Varchar
                    } else {
                        DataType::Text
                    };
                    (text_type, nullable)
                } else {
                    (input_type, nullable)
                }
            },
            Self::Substring | Self::Replace | Self::Left | Self::Right | Self::Repeat => {
                let nullable = arg_nullables.first().copied().unwrap_or(true);
                (input_type, nullable)
            },

            Self::Length
            | Self::CharLength
            | Self::OctetLength
            | Self::BitLength
            | Self::Position => {
                // MySQL returns BIGINT for these functions, others return INT
                let data_type = match dialect {
                    Dialect::MySQL => DataType::BigInt,
                    _ => DataType::Int,
                };
                let nullable = arg_nullables.first().copied().unwrap_or(true);
                (data_type, nullable)
            },

            Self::Ceil | Self::Floor => {
                let nullable = arg_nullables.first().copied().unwrap_or(true);
                // Only MySQL auto-converts non-numeric types for CEIL/FLOOR
                // SQLite requires numeric types
                if dialect == Dialect::MySQL && !input_type.is_numeric() {
                    (DataType::Double, nullable)
                } else {
                    (input_type, nullable)
                }
            },
            Self::Abs | Self::Round | Self::Truncate | Self::Mod => {
                let nullable = arg_nullables.first().copied().unwrap_or(true);
                // MySQL and SQLite auto-convert non-numeric types to numeric
                if matches!(dialect, Dialect::MySQL | Dialect::SQLite) && !input_type.is_numeric() {
                    (DataType::Double, nullable)
                } else {
                    (input_type, nullable)
                }
            },

            Self::Sqrt
            | Self::Exp
            | Self::Log
            | Self::Ln
            | Self::Log10
            | Self::Log2
            | Self::Power
            | Self::Random => (DataType::Double, true),

            Self::Sign => (DataType::Int, true),

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
            | Self::Extract => (DataType::Int, true),

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

    pub(crate) fn validate_argument_types(
        &self,
        dialect: Dialect,
        arg_types: &[DataType],
    ) -> Option<String> {
        match self {
            // String functions validation
            Self::Substring => {
                if dialect == Dialect::Postgres {
                    if let Some(first_arg) = arg_types.first() {
                        if !matches!(first_arg, DataType::Custom(_)) && !first_arg.is_text_like() {
                            return Some(
                                "expected a text or bytea argument for position 1 in PostgreSQL"
                                    .to_string(),
                            );
                        }
                    }
                }
                None
            },
            Self::Upper
            | Self::Lower
            | Self::Trim
            | Self::Ltrim
            | Self::Rtrim
            | Self::Length
            | Self::CharLength => {
                if dialect == Dialect::Postgres {
                    if let Some(first_arg) = arg_types.first() {
                        if !first_arg.is_text_like() {
                            return Some(format!(
                                "function {:?}(non-text) does not exist in PostgreSQL",
                                self
                            ));
                        }
                    }
                }
                None
            },
            // Numeric functions validation
            Self::Ceil | Self::Floor => {
                if dialect == Dialect::Postgres {
                    if let Some(first_arg) = arg_types.first() {
                        if !first_arg.is_numeric() {
                            return Some(format!(
                                "function {:?}(non-numeric) does not exist in PostgreSQL",
                                self
                            ));
                        }
                    }
                } else if dialect == Dialect::SQLite {
                    // SQLite does not support CEIL/FLOOR on non-numeric types
                    if let Some(first_arg) = arg_types.first() {
                        if !first_arg.is_numeric() {
                            return Some(format!(
                                "function {:?}(non-numeric) does not exist in SQLite",
                                self
                            ));
                        }
                    }
                }
                None
            },
            Self::Abs
            | Self::Round
            | Self::Truncate
            | Self::Sqrt
            | Self::Exp
            | Self::Log
            | Self::Ln
            | Self::Log10
            | Self::Log2
            | Self::Sign => {
                if dialect == Dialect::Postgres {
                    if let Some(first_arg) = arg_types.first() {
                        if !first_arg.is_numeric() {
                            return Some(format!(
                                "function {:?}(non-numeric) does not exist in PostgreSQL",
                                self
                            ));
                        }
                    }
                }
                None
            },
            Self::Power | Self::Mod => {
                if dialect == Dialect::Postgres {
                    for (i, arg_type) in arg_types.iter().enumerate() {
                        if !arg_type.is_numeric() {
                            return Some(format!(
                                "function {:?}() requires numeric arguments in PostgreSQL, argument {} is non-numeric",
                                self,
                                i + 1
                            ));
                        }
                    }
                }
                None
            },
            _ => None,
        }
    }
}
