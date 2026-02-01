//! SQL data type definitions.

/// Unified SQL data type representation across all dialects.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(Default)]
pub enum SqlType {
    // Integer types
    /// Small integer (2 bytes)
    SmallInt,
    /// Integer (4 bytes)
    Integer,
    /// Big integer (8 bytes)
    BigInt,

    // Floating point types
    /// Single precision floating point
    Real,
    /// Double precision floating point
    Double,
    /// Fixed precision decimal
    Decimal { precision: u8, scale: u8 },

    // String types
    /// Fixed-length character string
    Char(u32),
    /// Variable-length character string
    Varchar(Option<u32>),
    /// Text (unlimited length)
    Text,

    // Boolean
    /// Boolean type
    Boolean,

    // Date/Time types
    /// Date only
    Date,
    /// Time only (without timezone)
    Time,
    /// Timestamp (without timezone)
    Timestamp,
    /// Timestamp with timezone
    TimestampTz,

    // Binary types
    /// Binary large object
    Blob,
    /// PostgreSQL bytea
    Bytea,

    // JSON types
    /// JSON (text representation)
    Json,
    /// JSONB (binary representation, PostgreSQL)
    Jsonb,

    // Other types
    /// UUID type
    Uuid,
    /// Array of another type
    Array(Box<SqlType>),
    /// Custom/user-defined type
    Custom(String),
    /// Unknown type (for fallback)
    #[default]
    Unknown,
}

impl SqlType {
    /// Returns true if this type is numeric.
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            SqlType::SmallInt
                | SqlType::Integer
                | SqlType::BigInt
                | SqlType::Real
                | SqlType::Double
                | SqlType::Decimal { .. }
        )
    }

    /// Returns true if this type is a string type.
    pub fn is_string(&self) -> bool {
        matches!(self, SqlType::Char(_) | SqlType::Varchar(_) | SqlType::Text)
    }

    /// Returns true if this type is a date/time type.
    pub fn is_temporal(&self) -> bool {
        matches!(
            self,
            SqlType::Date | SqlType::Time | SqlType::Timestamp | SqlType::TimestampTz
        )
    }

    /// Returns the wider of two numeric types for arithmetic operations.
    pub fn wider_numeric(a: &SqlType, b: &SqlType) -> SqlType {
        use SqlType::*;
        match (a, b) {
            // If either is Double, result is Double
            (Double, _) | (_, Double) => Double,
            // If either is Real, result is Real (unless other is Double)
            (Real, _) | (_, Real) => Real,
            // Decimal propagates
            (
                Decimal {
                    precision: p1,
                    scale: s1,
                },
                Decimal {
                    precision: p2,
                    scale: s2,
                },
            ) => Decimal {
                precision: (*p1).max(*p2),
                scale: (*s1).max(*s2),
            },
            (Decimal { precision, scale }, _) | (_, Decimal { precision, scale }) => Decimal {
                precision: *precision,
                scale: *scale,
            },
            // BigInt is wider than Integer
            (BigInt, _) | (_, BigInt) => BigInt,
            // Integer is wider than SmallInt
            (Integer, _) | (_, Integer) => Integer,
            // Both SmallInt
            (SmallInt, SmallInt) => SmallInt,
            // Fallback
            _ => Unknown,
        }
    }
}


impl std::fmt::Display for SqlType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SqlType::SmallInt => write!(f, "SMALLINT"),
            SqlType::Integer => write!(f, "INTEGER"),
            SqlType::BigInt => write!(f, "BIGINT"),
            SqlType::Real => write!(f, "REAL"),
            SqlType::Double => write!(f, "DOUBLE"),
            SqlType::Decimal { precision, scale } => write!(f, "DECIMAL({},{})", precision, scale),
            SqlType::Char(n) => write!(f, "CHAR({})", n),
            SqlType::Varchar(Some(n)) => write!(f, "VARCHAR({})", n),
            SqlType::Varchar(None) => write!(f, "VARCHAR"),
            SqlType::Text => write!(f, "TEXT"),
            SqlType::Boolean => write!(f, "BOOLEAN"),
            SqlType::Date => write!(f, "DATE"),
            SqlType::Time => write!(f, "TIME"),
            SqlType::Timestamp => write!(f, "TIMESTAMP"),
            SqlType::TimestampTz => write!(f, "TIMESTAMPTZ"),
            SqlType::Blob => write!(f, "BLOB"),
            SqlType::Bytea => write!(f, "BYTEA"),
            SqlType::Json => write!(f, "JSON"),
            SqlType::Jsonb => write!(f, "JSONB"),
            SqlType::Uuid => write!(f, "UUID"),
            SqlType::Array(inner) => write!(f, "{}[]", inner),
            SqlType::Custom(name) => write!(f, "{}", name),
            SqlType::Unknown => write!(f, "UNKNOWN"),
        }
    }
}
