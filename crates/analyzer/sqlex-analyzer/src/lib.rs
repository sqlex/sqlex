use async_trait::async_trait;
pub mod extension;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnalyzerError {
    #[error("Execution failed: {0}")]
    ExecutionError(String),
    #[error("Analysis failed: {0}")]
    AnalysisError(String),
}

pub type Result<T> = std::result::Result<T, AnalyzerError>;

#[async_trait]
pub trait Analyzer: Send + Sync {
    /// Execute DDL or other state-changing SQL.
    async fn execute(&mut self, sql: &str) -> Result<()>;

    /// Analyze a query to determine its result set structure.
    async fn analyze(&self, sql: &str) -> Result<sqlex_common::types::ResultSet>;

    /// Get all tables schema
    async fn get_all_tables(&self) -> Result<Vec<sqlex_common::types::Table>>;
}
