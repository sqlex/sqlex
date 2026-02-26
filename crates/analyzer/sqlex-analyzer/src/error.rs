use std::error::Error as StdError;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnalyzerError {
    /// Represents deterministic, user-facing SQL analysis failures.
    ///
    /// This covers expected analyzer failures such as parse, name resolution,
    /// and type validation errors.
    ///
    /// Error code format is `<module><major><minor>`, for example `P0000` or
    /// `P0120`:
    /// - `<module>`: one uppercase letter representing analyzer module/feature.
    /// - `<major>`: two digits (00-99) for top-level category.
    /// - `<minor>`: two digits (00-99) for sub-category within the major group.
    #[error("[ANALYZE:{code}] {message}")]
    Analysis { code: &'static str, message: String },

    /// Represents a recognized analyzer path that is not implemented yet.
    ///
    /// This captures planned capabilities or dialect branches whose behavior is
    /// known but currently missing. The message carries static context.
    #[error("[TODO] {0}")]
    Todo(&'static str),

    /// Represents non-analysis failures without a structured diagnostic code.
    ///
    /// Typical cases include infrastructure and runtime integration failures
    /// where analysis cannot continue.
    #[error("Other error: {0}")]
    Other(String),
}

impl AnalyzerError {
    pub fn analysis(code: &'static str, message: impl Into<String>) -> Self {
        Self::Analysis {
            code,
            message: message.into(),
        }
    }

    pub fn todo(message: &'static str) -> Self {
        Self::Todo(message)
    }

    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }
}

impl From<String> for AnalyzerError {
    fn from(value: String) -> Self {
        Self::Other(value)
    }
}

impl From<&str> for AnalyzerError {
    fn from(value: &str) -> Self {
        Self::Other(value.to_owned())
    }
}

impl From<Box<dyn StdError + Send + Sync + 'static>> for AnalyzerError {
    fn from(value: Box<dyn StdError + Send + Sync + 'static>) -> Self {
        Self::Other(value.to_string())
    }
}
