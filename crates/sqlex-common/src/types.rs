use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Cardinality {
    /// Guarantees exactly one row will be returned
    ExactlyOne,
    /// Guarantees at least one row will be returned
    AtLeastOne,
    /// At most one row will be returned (0 or 1)
    AtMostOne,
    /// Row count is unknown
    #[default]
    Unknown,
}

impl Cardinality {
    /// Returns true if this cardinality guarantees at least one row
    pub fn guarantees_row(self) -> bool {
        matches!(self, Self::ExactlyOne | Self::AtLeastOne)
    }

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
