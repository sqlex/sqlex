use sqlex_common::DataType;

use super::typed::TypedExpr;

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

    // Fallback
    Custom(String),
}

impl ScalarFunction {
    pub fn from_name(name: &str) -> Option<Self> {
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
            "ROUND" => Some(Self::Round),
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

            // Don't treat unkwown as Custom here immediately, allow caller to decide
            // or we can allow Custom here.
            _ => None,
        }
    }

    pub fn result_type(&self, args: &[TypedExpr]) -> (DataType, bool) {
        let input_type = args
            .first()
            .map(|a| a.data_type.clone())
            .unwrap_or(DataType::Text); // Default fallback

        // Default nullability: result is nullable if any arg is nullable
        let any_arg_nullable = args.iter().any(|a| a.nullable);

        match self {
            // String -> Text
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
            | Self::Repeat => (DataType::Text, any_arg_nullable),

            // String -> Int
            Self::Length
            | Self::CharLength
            | Self::CharacterLength
            | Self::OctetLength
            | Self::BitLength
            | Self::Position
            | Self::Strpos => (DataType::Int, any_arg_nullable),

            // Numeric -> Same as input (or promote?)
            Self::Abs
            | Self::Ceil
            | Self::Ceiling
            | Self::Floor
            | Self::Round
            | Self::Truncate
            | Self::Trunc
            | Self::Mod => (input_type, any_arg_nullable),

            // Numeric -> Double
            Self::Sqrt
            | Self::Exp
            | Self::Log
            | Self::Ln
            | Self::Log10
            | Self::Log2
            | Self::Power
            | Self::Pow
            | Self::Random
            | Self::Rand => (DataType::Double, any_arg_nullable),

            // Numeric -> Int
            Self::Sign => (DataType::Int, any_arg_nullable),

            // Date/Time
            Self::Now | Self::CurrentTimestamp => (DataType::Timestamp, any_arg_nullable),
            Self::CurrentDate => (DataType::Date, any_arg_nullable),
            Self::CurrentTime => (DataType::Time, any_arg_nullable),
            Self::Date => (DataType::Date, any_arg_nullable),
            Self::Time => (DataType::Time, any_arg_nullable),

            Self::Year
            | Self::Month
            | Self::Day
            | Self::Hour
            | Self::Minute
            | Self::Second
            | Self::Extract => (DataType::Int, any_arg_nullable),

            // JSON
            Self::JsonObject | Self::JsonArray | Self::ToJson | Self::ToJsonb => {
                (DataType::Json, any_arg_nullable)
            },

            // Control
            Self::Coalesce => {
                // Return type is first arg type (simplified)
                // Nullable if ALL args are nullable
                let all_nullable = args.iter().all(|a| a.nullable);
                (input_type, all_nullable)
            },
            Self::Nullif => {
                // Returns same type as first arg
                // Nullable because it returns NULL if args are equal
                (input_type, true)
            },
            Self::Ifnull | Self::Nvl => {
                // Like Coalesce but usually 2 args
                let all_nullable = args.iter().all(|a| a.nullable);
                (input_type, all_nullable)
            },

            // Conversion
            Self::Cast | Self::Convert => {
                // Depends on target type which we don't have here easily
                // For now, identity
                (input_type, any_arg_nullable)
            },

            Self::Custom(_) => (DataType::Custom("unknown".to_string()), true),
        }
    }
}
