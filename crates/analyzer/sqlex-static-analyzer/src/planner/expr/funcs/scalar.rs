use sqlex_common::DataType;
use sqlparser::dialect::Dialect;

use crate::planner::expr::{Expression, ExpressionNode};

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

    // Fallback / Unknown
    Unknown,
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

    /// Infer type for the new Expression system (takes DataType slices)
    pub fn infer_type(&self, arg_types: &[DataType]) -> (DataType, bool) {
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

            Self::Coalesce | Self::Ifnull | Self::Nvl => (input_type, true),
            Self::Nullif => (input_type, true),

            Self::Cast | Self::Convert => (input_type, true),

            Self::Custom(_) | Self::Unknown => (DataType::Custom("unknown".to_string()), true),
        }
    }
}

/// Scalar function expression
#[derive(Debug, Clone)]
pub struct ScalarFunctionExpr {
    pub name: String,
    pub function: ScalarFunction,
    pub args: Vec<Box<dyn Expression>>,
    pub return_type: DataType,
    pub is_nullable: bool,
}

impl ExpressionNode for ScalarFunctionExpr {
    fn data_type(&self) -> DataType {
        self.return_type.clone()
    }

    fn nullable(&self) -> bool {
        self.is_nullable
    }
}

impl ScalarFunctionExpr {
    /// Build a scalar function expression
    pub fn build(
        _dialect: &dyn Dialect,
        name: String,
        args: Vec<Box<dyn Expression>>,
    ) -> Box<dyn Expression> {
        // Try to match the function
        let function = ScalarFunction::from_name(&name).unwrap_or(ScalarFunction::Unknown);

        // Infer return type using the helper
        let arg_types: Vec<DataType> = args.iter().map(|e| e.data_type()).collect();
        let (return_type, is_nullable) = function.infer_type(&arg_types);

        Box::new(ScalarFunctionExpr {
            name,
            function,
            args,
            return_type,
            is_nullable,
        })
    }
}
