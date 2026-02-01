//! PostgreSQL backend using testcontainers.

use async_trait::async_trait;
use sqlx::{Column, Executor, PgPool, postgres::PgPoolOptions};
use testcontainers::{ContainerAsync, runners::AsyncRunner};
use testcontainers_modules::postgres::Postgres;

use super::{ColumnMetadata, DatabaseBackend, ParamMetadata, QueryMetadata};
use crate::{Result, config::Dialect};

/// PostgreSQL database backend.
pub struct PostgresBackend {
    pool: PgPool,
    #[allow(dead_code)]
    container: Option<ContainerAsync<Postgres>>,
}

impl PostgresBackend {
    /// Start a PostgreSQL container.
    pub async fn start_container() -> Result<ContainerAsync<Postgres>> {
        let container = Postgres::default().start().await.map_err(|e| {
            crate::Error::Config(format!("Failed to start PostgreSQL container: {}", e))
        })?;
        Ok(container)
    }

    /// Connect to an existing PostgreSQL database.
    pub async fn connect(host: &str, port: u16, db: &str) -> Result<Self> {
        let connection_string = format!("postgres://postgres:postgres@{}:{}/{}", host, port, db);

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await?;

        Ok(Self {
            pool,
            container: None,
        })
    }

    /// Create a new database on the PostgreSQL instance.
    pub async fn create_database(host: &str, port: u16, db_name: &str) -> Result<()> {
        let connection_string = format!("postgres://postgres:postgres@{}:{}/postgres", host, port);
        // We use a separate pool for admin tasks
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&connection_string)
            .await?;

        // Use raw_sql to avoid prepared statement issues with CREATE DATABASE if any
        sqlx::query(&format!("CREATE DATABASE \"{}\"", db_name))
            .execute(&pool)
            .await?;

        Ok(())
    }

    /// Create a new PostgreSQL backend with a testcontainer.
    pub async fn new() -> Result<Self> {
        // Start PostgreSQL container
        let container = Self::start_container().await?;

        let host = container
            .get_host()
            .await
            .map_err(|e| crate::Error::Config(format!("Failed to get container host: {}", e)))?;
        let port = container
            .get_host_port_ipv4(5432)
            .await
            .map_err(|e| crate::Error::Config(format!("Failed to get container port: {}", e)))?;

        let connection_string = format!("postgres://postgres:postgres@{}:{}/postgres", host, port);

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await?;

        Ok(Self {
            pool,
            container: Some(container),
        })
    }
}

#[async_trait]
impl DatabaseBackend for PostgresBackend {
    fn dialect(&self) -> Dialect {
        Dialect::Postgresql
    }

    async fn execute_migration(&self, sql: &str) -> Result<()> {
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
            .map(
                |p: sqlx::Either<&[sqlx::postgres::PgTypeInfo], usize>| match p {
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
                },
            )
            .unwrap_or_default();

        Ok(QueryMetadata { columns, params })
    }

    async fn cleanup(&self) -> Result<()> {
        // Container will be cleaned up when dropped
        Ok(())
    }
}
