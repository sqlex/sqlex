use serde::{Deserialize, Serialize};

/// Row-count bound of a query result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Cardinality {
    /// The query result is guaranteed to contain no rows.
    ExactlyZero,
    /// The query result is guaranteed to contain exactly one row.
    ExactlyOne,
    /// The query result may contain zero or one row.
    AtMostOne,
    /// The query result is guaranteed to contain at least one row.
    OneOrMore,
    /// The query result may contain any number of rows, including zero.
    #[default]
    ZeroOrMore,
}

impl Cardinality {
    /// Returns true if this cardinality guarantees at most one row
    pub fn is_single_row(self) -> bool {
        matches!(self, Self::ExactlyOne | Self::AtMostOne)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    // Integers
    Bool,
    TinyInt,
    UnsignedTinyInt,
    SmallInt,
    UnsignedSmallInt,
    Int,
    UnsignedInt,
    BigInt,
    UnsignedBigInt,

    // Floats
    Float,
    Double,
    Decimal,

    // Strings
    Char,
    Varchar,
    Text,

    // Time
    Date,
    Time,
    DateTime,
    Timestamp,

    // Others
    Uuid,
    Json,
    Binary,

    // Complex
    Array(Box<DataType>),

    // Fallback
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: DataType,
    pub nullability: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultSet {
    pub columns: Vec<ColumnInfo>,
    pub cardinality: Cardinality,
}
