//! Parse error definitions.

use thiserror::Error;

/// Errors that can occur during SQL parsing.
#[derive(Debug, Error)]
pub enum ParseError {
    /// SQL syntax error
    #[error("SQL syntax error: {0}")]
    Syntax(String),

    /// Empty input
    #[error("Empty SQL input")]
    EmptyInput,

    /// Multiple statements when only one expected
    #[error("Expected single statement, got {0}")]
    MultipleStatements(usize),

    /// Unsupported dialect
    #[error("Unsupported dialect: {0}")]
    UnsupportedDialect(String),
}

impl From<sqlparser::parser::ParserError> for ParseError {
    fn from(err: sqlparser::parser::ParserError) -> Self {
        ParseError::Syntax(err.to_string())
    }
}
