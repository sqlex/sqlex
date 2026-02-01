//! SQL data type conversion utilities.

use sqlex_types::{Dialect, SqlType};
use sqlparser::ast::DataType as SqlDataType;

/// Convert sqlparser DataType to our SqlType.
pub fn convert_data_type(data_type: &SqlDataType, _dialect: Dialect) -> SqlType {
    match data_type {
        // Integer types
        SqlDataType::SmallInt(_) | SqlDataType::Int2(_) => SqlType::SmallInt,
        SqlDataType::Int(_) | SqlDataType::Integer(_) | SqlDataType::Int4(_) => SqlType::Integer,
        SqlDataType::BigInt(_) | SqlDataType::Int8(_) => SqlType::BigInt,

        // Serial types (PostgreSQL)
        SqlDataType::Custom(name, _) => {
            let name_str = name.to_string().to_uppercase();
            match name_str.as_str() {
                "SERIAL" | "SERIAL4" => SqlType::Integer,
                "BIGSERIAL" | "SERIAL8" => SqlType::BigInt,
                "SMALLSERIAL" | "SERIAL2" => SqlType::SmallInt,
                _ => SqlType::Custom(name.to_string()),
            }
        },

        // Floating point types
        SqlDataType::Real | SqlDataType::Float4 => SqlType::Real,
        SqlDataType::Double(_) | SqlDataType::DoublePrecision | SqlDataType::Float8 => {
            SqlType::Double
        },
        SqlDataType::Float(precision) => {
            // Float with precision <= 24 is Real, otherwise Double
            match precision {
                Some(p) if *p <= 24 => SqlType::Real,
                _ => SqlType::Double,
            }
        },

        // Decimal types
        SqlDataType::Decimal(info) | SqlDataType::Numeric(info) => {
            let (precision, scale) = match info {
                sqlparser::ast::ExactNumberInfo::PrecisionAndScale(p, s) => (*p as u8, *s as u8),
                sqlparser::ast::ExactNumberInfo::Precision(p) => (*p as u8, 0),
                sqlparser::ast::ExactNumberInfo::None => (38, 0),
            };
            SqlType::Decimal { precision, scale }
        },

        // String types
        SqlDataType::Char(len) | SqlDataType::Character(len) => {
            let n = extract_char_length(len).unwrap_or(1);
            SqlType::Char(n)
        },
        SqlDataType::Varchar(len) | SqlDataType::CharacterVarying(len) => {
            let n = extract_char_length(len);
            SqlType::Varchar(n)
        },
        SqlDataType::Text | SqlDataType::String(_) => SqlType::Text,

        // Boolean
        SqlDataType::Boolean | SqlDataType::Bool => SqlType::Boolean,

        // Date/Time types
        SqlDataType::Date => SqlType::Date,
        SqlDataType::Time(_, _tz) => SqlType::Time,
        SqlDataType::Timestamp(_, tz) => match tz {
            sqlparser::ast::TimezoneInfo::WithTimeZone => SqlType::TimestampTz,
            _ => SqlType::Timestamp,
        },
        SqlDataType::Datetime(_) => SqlType::Timestamp,

        // Binary types
        SqlDataType::Blob(_) | SqlDataType::Binary(_) | SqlDataType::Varbinary(_) => SqlType::Blob,
        SqlDataType::Bytea => SqlType::Bytea,

        // JSON types
        SqlDataType::JSON => SqlType::Json,
        SqlDataType::JSONB => SqlType::Jsonb,

        // UUID
        SqlDataType::Uuid => SqlType::Uuid,

        // Array types
        SqlDataType::Array(arr_def) => match arr_def {
            sqlparser::ast::ArrayElemTypeDef::AngleBracket(inner) => {
                SqlType::Array(Box::new(convert_data_type(inner, _dialect)))
            },
            sqlparser::ast::ArrayElemTypeDef::SquareBracket(inner, _) => {
                SqlType::Array(Box::new(convert_data_type(inner, _dialect)))
            },
            sqlparser::ast::ArrayElemTypeDef::Parenthesis(inner) => {
                SqlType::Array(Box::new(convert_data_type(inner, _dialect)))
            },
            sqlparser::ast::ArrayElemTypeDef::None => SqlType::Array(Box::new(SqlType::Unknown)),
        },

        // Fallback
        _ => SqlType::Unknown,
    }
}

/// Extract length from CharacterLength enum
fn extract_char_length(len: &Option<sqlparser::ast::CharacterLength>) -> Option<u32> {
    len.as_ref().and_then(|cl| match cl {
        sqlparser::ast::CharacterLength::IntegerLength { length, .. } => Some(*length as u32),
        sqlparser::ast::CharacterLength::Max => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_integer_types() {
        assert_eq!(
            convert_data_type(&SqlDataType::Integer(None), Dialect::PostgreSQL),
            SqlType::Integer
        );
        assert_eq!(
            convert_data_type(&SqlDataType::BigInt(None), Dialect::PostgreSQL),
            SqlType::BigInt
        );
    }

    #[test]
    fn test_convert_varchar() {
        use sqlparser::ast::CharacterLength;
        let varchar_100 = SqlDataType::Varchar(Some(CharacterLength::IntegerLength {
            length: 100,
            unit: None,
        }));
        assert_eq!(
            convert_data_type(&varchar_100, Dialect::PostgreSQL),
            SqlType::Varchar(Some(100))
        );
    }

    #[test]
    fn test_convert_boolean() {
        assert_eq!(
            convert_data_type(&SqlDataType::Boolean, Dialect::PostgreSQL),
            SqlType::Boolean
        );
    }
}
