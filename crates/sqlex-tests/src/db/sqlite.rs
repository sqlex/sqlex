//! SQLite backend using in-memory database.

use async_trait::async_trait;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool, Column, Executor};

use super::{ColumnMetadata, DatabaseBackend, ParamMetadata, QueryMetadata};
use crate::{config::Dialect, Result};

/// SQLite database backend (in-memory).
pub struct SqliteBackend {
    pool: SqlitePool,
}

impl SqliteBackend {
    /// Create a new SQLite backend with an in-memory database.
    pub async fn new() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(":memory:")
            .await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl DatabaseBackend for SqliteBackend {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }

    async fn execute_migration(&self, sql: &str) -> Result<()> {
        // SQLite can handle multiple statements with raw_sql
        sqlx::raw_sql(sql).execute(&self.pool).await?;
        Ok(())
    }

    async fn describe_query(&self, sql: &str) -> Result<QueryMetadata> {
        let describe = self.pool.describe(sql).await?;

        let columns = describe
            .columns
            .iter()
            .zip(describe.nullable.iter())
            .map(|(col, nullable)| ColumnMetadata {
                name: col.name().to_string(),
                type_name: col.type_info().to_string(),
                nullable: nullable.unwrap_or(true),
            })
            .collect();

        let params = describe
            .parameters()
            .map(|p: sqlx::Either<&[sqlx::sqlite::SqliteTypeInfo], usize>| match p {
                sqlx::Either::Left(types) => types
                    .iter()
                    .enumerate()
                    .map(|(i, t)| ParamMetadata {
                        position: i + 1,
                        type_name: Some(t.to_string()),
                    })
                    .collect(),
                sqlx::Either::Right(count) => (0..count)
                    .map(|i| ParamMetadata {
                        position: i + 1,
                        type_name: None,
                    })
                    .collect(),
            })
            .unwrap_or_default();

        Ok(QueryMetadata { columns, params })
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}
