//! Database connection and metadata extraction.

pub mod mysql;
pub mod postgres;
pub mod sqlite;

use async_trait::async_trait;

use crate::{Result, config::Dialect};

/// Metadata for a single column in a query result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnMetadata {
    /// Column name.
    pub name: String,
    /// Column type as reported by the database.
    pub type_name: String,
    /// Whether the column can be NULL.
    pub nullable: bool,
}

/// Metadata for a query parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamMetadata {
    /// Parameter position (1-indexed).
    pub position: usize,
    /// Parameter type as reported by the database.
    pub type_name: Option<String>,
}

/// Complete metadata for a prepared query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryMetadata {
    /// Columns in the result set.
    pub columns: Vec<ColumnMetadata>,
    /// Parameters in the query.
    pub params: Vec<ParamMetadata>,
}

/// Trait for database backends that can execute migrations and describe queries.
#[async_trait]
pub trait DatabaseBackend: Send + Sync {
    /// Get the dialect for this backend.
    fn dialect(&self) -> Dialect;

    /// Execute a migration script.
    async fn execute_migration(&self, sql: &str) -> Result<()>;

    /// Describe a query and return its metadata.
    async fn describe_query(&self, sql: &str) -> Result<QueryMetadata>;

    /// Clean up resources (drop tables, close connections, etc.)
    async fn cleanup(&self) -> Result<()>;
}

/// Create a database backend for the specified dialect.
pub async fn create_backend(dialect: Dialect) -> Result<Box<dyn DatabaseBackend>> {
    match dialect {
        Dialect::Postgresql => {
            let backend = postgres::PostgresBackend::new().await?;
            Ok(Box::new(backend))
        },
        Dialect::Mysql => {
            let backend = mysql::MysqlBackend::new().await?;
            Ok(Box::new(backend))
        },
        Dialect::Sqlite => {
            let backend = sqlite::SqliteBackend::new().await?;
            Ok(Box::new(backend))
        },
    }
}
