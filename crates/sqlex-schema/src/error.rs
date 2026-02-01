//! Schema error definitions.

use thiserror::Error;

/// Errors that can occur during schema operations.
#[derive(Debug, Error)]
pub enum SchemaError {
    /// Table already exists
    #[error("Table already exists: {0}")]
    TableAlreadyExists(String),

    /// Table not found
    #[error("Table not found: {0}")]
    TableNotFound(String),

    /// Column already exists
    #[error("Column already exists: {table}.{column}")]
    ColumnAlreadyExists { table: String, column: String },

    /// Column not found
    #[error("Column not found: {table}.{column}")]
    ColumnNotFound { table: String, column: String },

    /// Parse error
    #[error("Parse error: {0}")]
    Parse(#[from] sqlex_parser::ParseError),

    /// Unsupported statement
    #[error("Unsupported statement: {0}")]
    UnsupportedStatement(String),
}
