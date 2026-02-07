use serde::{Deserialize, Serialize};

use crate::types::{Cardinality, ColumnInfo, DataType, Table};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationUnit {
    pub tables: Vec<Table>,
    pub queries: Vec<QueryDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryDescriptor {
    /// The name of the query (e.g., from filename or annotation)
    pub name: String,

    /// The original or processed SQL
    pub sql: String,

    /// Input parameters for the query
    pub params: Vec<ParameterDescriptor>,

    /// Cardinality of the query result
    pub cardinality: Cardinality,

    /// Output columns of the query result
    pub columns: Vec<ColumnInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDescriptor {
    pub name: String,
    pub type_info: DataType,
    pub nullable: bool,
}
