//! Error types for the test framework.

use thiserror::Error;

/// Result type alias for sqlex-tests.
pub type Result<T> = std::result::Result<T, Error>;

/// Error types that can occur during test execution.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Migration parse error: {0}")]
    MigrationParse(String),

    #[error("Query analysis error: {0}")]
    Analysis(String),

    #[error("Metadata mismatch: {message}")]
    MetadataMismatch { message: String },
}
