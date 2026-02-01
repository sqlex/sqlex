//! MySQL backend using testcontainers.

use async_trait::async_trait;
use sqlx::{Column, Executor, MySqlPool, mysql::MySqlPoolOptions};
use testcontainers::{ContainerAsync, runners::AsyncRunner};
use testcontainers_modules::mysql::Mysql;

use super::{ColumnMetadata, DatabaseBackend, ParamMetadata, QueryMetadata};
use crate::{Result, config::Dialect};

/// MySQL database backend.
pub struct MysqlBackend {
    pool: MySqlPool,
    #[allow(dead_code)]
    container: ContainerAsync<Mysql>,
}

impl MysqlBackend {
    /// Create a new MySQL backend with a testcontainer.
    pub async fn new() -> Result<Self> {
        // Start MySQL container
        let container = Mysql::default()
            .start()
            .await
            .map_err(|e| crate::Error::Config(format!("Failed to start MySQL container: {}", e)))?;

        let host = container
            .get_host()
            .await
            .map_err(|e| crate::Error::Config(format!("Failed to get container host: {}", e)))?;
        let port = container
            .get_host_port_ipv4(3306)
            .await
            .map_err(|e| crate::Error::Config(format!("Failed to get container port: {}", e)))?;

        let connection_string = format!("mysql://root@{}:{}/test", host, port);

        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await?;

        Ok(Self { pool, container })
    }
}

#[async_trait]
impl DatabaseBackend for MysqlBackend {
    fn dialect(&self) -> Dialect {
        Dialect::Mysql
    }

    async fn execute_migration(&self, sql: &str) -> Result<()> {
        // MySQL doesn't support multiple statements in one query easily,
        // so we split by semicolons and execute each statement.
        // We need to be careful not to split on semicolons inside string literals.
        let mut statements = Vec::new();
        let mut current = String::new();
        let mut in_string = false;
        let mut escape = false;

        for c in sql.chars() {
            if escape {
                current.push(c);
                escape = false;
                continue;
            }

            if c == '\'' {
                in_string = !in_string;
                current.push(c);
            } else if c == '\\' && in_string {
                escape = true;
                current.push(c);
            } else if c == ';' && !in_string {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    statements.push(trimmed.to_string());
                }
                current.clear();
            } else {
                current.push(c);
            }
        }

        let trimmed = current.trim();
        if !trimmed.is_empty() {
            statements.push(trimmed.to_string());
        }

        for statement in statements {
            sqlx::raw_sql(&statement).execute(&self.pool).await?;
        }
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
                |p: sqlx::Either<&[sqlx::mysql::MySqlTypeInfo], usize>| match p {
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
        Ok(())
    }
}
