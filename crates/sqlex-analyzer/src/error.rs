//! Analyzer error definitions.

use thiserror::Error;

/// Errors that can occur during query analysis.
#[derive(Debug, Error)]
pub enum AnalyzeError {
    /// Parse error
    #[error("Parse error: {0}")]
    Parse(#[from] sqlex_parser::ParseError),

    /// Not a SELECT query
    #[error("Expected SELECT query")]
    NotASelectQuery,

    /// Unknown table
    #[error("Unknown table: {0}")]
    UnknownTable(String),

    /// Unknown column
    #[error("Unknown column: {0}")]
    UnknownColumn(String),

    /// Ambiguous column reference
    #[error("Ambiguous column reference: {0}")]
    AmbiguousColumn(String),

    /// Unsupported feature
    #[error("Unsupported: {0}")]
    Unsupported(String),
}
