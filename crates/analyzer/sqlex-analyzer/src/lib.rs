use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnalyzerError {
    #[error("Execution failed: {0}")]
    ExecutionError(String),
    #[error("Analysis failed: {0}")]
    AnalysisError(String),
}

pub type Result<T> = std::result::Result<T, AnalyzerError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    // Integers
    Bool,
    TinyInt,
    SmallInt,
    Int,
    BigInt,

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
pub struct ResultSet {
    pub columns: Vec<ColumnInfo>,
}

#[async_trait]
pub trait Analyzer: Send + Sync {
    /// Execute DDL or other state-changing SQL.
    async fn execute(&mut self, sql: &str) -> Result<()>;

    /// Analyze a query to determine its result set structure.
    async fn analyze(&self, sql: &str) -> Result<ResultSet>;
}
