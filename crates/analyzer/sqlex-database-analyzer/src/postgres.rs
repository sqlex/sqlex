use async_trait::async_trait;
use rand::{Rng, distributions::Alphanumeric};
use sqlex_analyzer::{Analyzer, AnalyzerError, Result};
use sqlex_common::{
    dialect::Dialect,
    types::{ColumnInfo, DataType, ResultSet, Table},
};
use sqlx::{
    Column, Executor, Row, Statement, TypeInfo,
    postgres::{PgPool, PgPoolOptions},
};
use testcontainers::{GenericImage, ImageExt, runners::AsyncRunner};

use crate::{
    container_pool::{ContainerInfo, ContainerPool},
    docker_raw::{DIALECT_LABEL_KEY, MANAGED_LABEL_KEY, MANAGED_LABEL_VALUE},
    utils::parse_retry_connect,
};

pub struct PostgresDatabaseAnalyzer {
    pool: PgPool,
    db_name: String,
    admin_url: String,
}

impl PostgresDatabaseAnalyzer {
    pub async fn new() -> Result<Self> {
        let shared = ContainerPool::global()
            .get_or_create_container(Dialect::Postgres, || async {
                let password: String = rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(16)
                    .map(char::from)
                    .collect();

                let image = GenericImage::new("postgres", "16-alpine")
                    .with_env_var("POSTGRES_PASSWORD", &password)
                    .with_label(MANAGED_LABEL_KEY, MANAGED_LABEL_VALUE)
                    .with_label(DIALECT_LABEL_KEY, "postgres");

                let container = image.start().await.map_err(|e| {
                    AnalyzerError::ExecutionError(format!(
                        "Failed to start postgres container: {}",
                        e
                    ))
                })?;

                let container_id = container.id().to_string();
                let host = container
                    .get_host()
                    .await
                    .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?
                    .to_string();
                let port = container
                    .get_host_port_ipv4(5432)
                    .await
                    .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;

                let url = format!(
                    "postgres://postgres:{}@{}:{}/postgres",
                    password, host, port
                );
                parse_retry_connect(|| PgPoolOptions::new().connect(&url)).await?;

                std::mem::forget(container);

                Ok(ContainerInfo {
                    container_id,
                    dialect: Dialect::Postgres,
                    host,
                    port,
                    password,
                })
            })
            .await?;

        // Create a unique database for this analyzer instance
        let db_name = format!(
            "sqlex_{}",
            rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(8)
                .map(char::from)
                .collect::<String>()
                .to_lowercase()
        );
        let admin_url = format!(
            "postgres://postgres:{}@{}:{}/postgres",
            shared.password, shared.host, shared.port
        );
        let admin_pool = PgPoolOptions::new()
            .connect(&admin_url)
            .await
            .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;

        admin_pool
            .execute(format!("CREATE DATABASE {}", db_name).as_str())
            .await
            .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;

        // Connect to the new database
        let url = format!(
            "postgres://postgres:{}@{}:{}/{}",
            shared.password, shared.host, shared.port, db_name
        );
        let pool = PgPoolOptions::new()
            .connect(&url)
            .await
            .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;

        Ok(Self {
            pool,
            db_name,
            admin_url,
        })
    }
}

#[async_trait]
impl Analyzer for PostgresDatabaseAnalyzer {
    async fn execute(&mut self, sql: &str) -> Result<()> {
        self.pool
            .execute(sql)
            .await
            .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;
        Ok(())
    }

    async fn analyze(&self, sql: &str) -> Result<ResultSet> {
        let stmt = self
            .pool
            .prepare(sql)
            .await
            .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;

        let mut columns = Vec::new();
        for col in stmt.columns() {
            let name = col.name().to_string();
            let data_type = {
                let type_info = col.type_info();
                map_udt(&type_info.name().to_lowercase())
            };
            columns.push(ColumnInfo {
                name,
                data_type,
                nullability: true,
            });
        }

        Ok(ResultSet {
            columns,
            cardinality: sqlex_common::types::Cardinality::Unknown,
        })
    }

    async fn get_all_tables(&self) -> Result<Vec<Table>> {
        let query = r#"
            SELECT c.table_name, c.column_name, c.udt_name, c.is_nullable
            FROM information_schema.columns c
            JOIN information_schema.tables t ON c.table_name = t.table_name AND c.table_schema = t.table_schema
            WHERE c.table_schema = 'public'
              AND t.table_type = 'BASE TABLE'
            ORDER BY c.table_name, c.ordinal_position
        "#;

        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;

        let mut tables_map: std::collections::HashMap<String, Vec<ColumnInfo>> =
            std::collections::HashMap::new();
        let mut table_order = Vec::new();

        for row in rows {
            let table_name: String = row
                .try_get("table_name")
                .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;
            let column_name: String = row
                .try_get("column_name")
                .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;
            let udt_name: String = row
                .try_get("udt_name")
                .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;
            let is_nullable: String = row
                .try_get("is_nullable")
                .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;

            let data_type = map_udt(&udt_name);
            let nullability = is_nullable == "YES";

            let col_info = ColumnInfo {
                name: column_name,
                data_type,
                nullability,
            };

            if !tables_map.contains_key(&table_name) {
                table_order.push(table_name.clone());
                tables_map.insert(table_name.clone(), Vec::new());
            }
            if let Some(columns) = tables_map.get_mut(&table_name) {
                columns.push(col_info);
            } else {
                return Err(AnalyzerError::AnalysisError(format!(
                    "Missing table entry while collecting Postgres metadata: {}",
                    table_name
                )));
            }
        }

        let mut tables = Vec::with_capacity(table_order.len());
        for name in table_order {
            let columns = tables_map.remove(&name).ok_or_else(|| {
                AnalyzerError::AnalysisError(format!(
                    "Missing collected columns for Postgres table: {}",
                    name
                ))
            })?;
            tables.push(Table { columns, name });
        }

        Ok(tables)
    }
}

impl Drop for PostgresDatabaseAnalyzer {
    fn drop(&mut self) {
        let db_name = self.db_name.clone();
        let admin_url = self.admin_url.clone();

        // Try to drop the database in a blocking manner
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                if let Ok(admin_pool) = PgPoolOptions::new().connect(&admin_url).await {
                    // Terminate existing connections before dropping
                    let terminate_query = format!(
                        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}' AND pid <> pg_backend_pid()",
                        db_name
                    );
                    let _ = admin_pool.execute(terminate_query.as_str()).await;

                    let _ = admin_pool
                        .execute(format!("DROP DATABASE IF EXISTS {}", db_name).as_str())
                        .await;
                }
            });
        }
    }
}

fn map_udt(udt: &str) -> DataType {
    match udt {
        "bool" | "boolean" => DataType::Bool,
        "int2" | "smallint" => DataType::SmallInt(false),
        "int4" | "integer" | "int" => DataType::Int(false),
        "int8" | "bigint" => DataType::BigInt(false),
        "float4" | "real" => DataType::Float,
        "float8" | "double precision" => DataType::Double,
        "numeric" | "decimal" => DataType::Decimal,
        "varchar" => DataType::Varchar,
        "bpchar" | "char" => DataType::Char,
        "text" => DataType::Text,
        "date" => DataType::Date,
        "time" | "timetz" => DataType::Time,
        "timestamp" => DataType::DateTime,
        "timestamptz" => DataType::Timestamp,
        "uuid" => DataType::Uuid,
        "json" | "jsonb" => DataType::Json,
        "bytea" => DataType::Binary,
        _ => DataType::Custom(udt.to_string()),
    }
}
