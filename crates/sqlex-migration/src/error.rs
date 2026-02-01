//! Migration error definitions.

use thiserror::Error;

/// Errors that can occur during migration operations.
#[derive(Debug, Error)]
pub enum MigrationError {
    /// Failed to read migration directory
    #[error("Failed to read migration directory: {0}")]
    ReadDir(#[from] std::io::Error),

    /// Invalid migration filename
    #[error("Invalid migration filename: {filename}. Expected format: V1__description.sql or 001_description.sql")]
    InvalidFilename { filename: String },

    /// Duplicate migration version
    #[error("Duplicate migration version {version}: {first} and {second}")]
    DuplicateVersion {
        version: u64,
        first: String,
        second: String,
    },

    /// Failed to apply migration
    #[error("Failed to apply migration V{version}__{name}: {reason}")]
    Apply {
        version: u64,
        name: String,
        reason: String,
    },
}
