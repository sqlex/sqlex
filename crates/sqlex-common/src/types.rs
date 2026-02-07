use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    // Integers
    Bool,
    TinyInt(bool),
    SmallInt(bool),
    Int(bool),
    BigInt(bool),

    // Floats
    Float,
    Double,
    Decimal,

    // Strings
    Char(Option<u32>),
    Varchar(Option<u32>),
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
}
