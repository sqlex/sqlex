use async_trait::async_trait;
use sqlex_common::types::{ResultSet, Table};

use crate::error::AnalyzerError;

pub mod error;
pub mod extension;

pub type Result<T> = std::result::Result<T, AnalyzerError>;

#[async_trait]
pub trait Analyzer: Send + Sync {
    /// Execute DDL or other state-changing SQL.
    async fn execute(&mut self, sql: &str) -> Result<()>;

    /// Analyze a query to determine its result set structure.
    async fn analyze(&self, sql: &str) -> Result<ResultSet>;

    /// Get all tables schema
    async fn get_all_tables(&self) -> Result<Vec<Table>>;
}
