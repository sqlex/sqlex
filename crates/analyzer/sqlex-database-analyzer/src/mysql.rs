use async_trait::async_trait;
use rand::{Rng, distributions::Alphanumeric};
use sqlex_analyzer::{Analyzer, AnalyzerError, Result};
use sqlex_common::{
    dialect::Dialect,
    types::{ColumnInfo, DataType, ResultSet, Table},
};
use sqlx::{
    Column, Executor, Row, Statement, TypeInfo,
    mysql::{MySqlPool, MySqlPoolOptions},
};
use testcontainers::{GenericImage, ImageExt, runners::AsyncRunner};

use crate::{
    container_pool::{ContainerInfo, ContainerPool},
    docker_raw::{DIALECT_LABEL_KEY, MANAGED_LABEL_KEY, MANAGED_LABEL_VALUE},
    utils::parse_retry_connect,
};

pub struct MySqlDatabaseAnalyzer {
    pool: MySqlPool,
    db_name: String,
    admin_url: String,
}

impl MySqlDatabaseAnalyzer {
    pub async fn new() -> Result<Self> {
        let shared = ContainerPool::global()
            .get_or_create_container(Dialect::MySQL, || async {
                let password: String = rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(16)
                    .map(char::from)
                    .collect();

                let image = GenericImage::new("mysql", "8")
                    .with_env_var("MYSQL_ROOT_PASSWORD", &password)
                    .with_label(MANAGED_LABEL_KEY, MANAGED_LABEL_VALUE)
                    .with_label(DIALECT_LABEL_KEY, "mysql");

                let container = image.start().await.map_err(|e| {
                    AnalyzerError::ExecutionError(format!("Failed to start mysql container: {}", e))
                })?;

                let container_id = container.id().to_string();
                let host = container
                    .get_host()
                    .await
                    .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?
                    .to_string();
                let port = container
                    .get_host_port_ipv4(3306)
                    .await
                    .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;

                let url = format!("mysql://root:{}@{}:{}", password, host, port);
                parse_retry_connect(|| MySqlPoolOptions::new().connect(&url)).await?;

                std::mem::forget(container);

                Ok(ContainerInfo {
                    container_id,
                    dialect: Dialect::MySQL,
                    host,
                    port,
                    password,
                })
            })
            .await?;

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
            "mysql://root:{}@{}:{}",
            shared.password, shared.host, shared.port
        );
        let admin_pool = MySqlPoolOptions::new()
            .connect(&admin_url)
            .await
            .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;

        admin_pool
            .execute(format!("CREATE DATABASE {}", db_name).as_str())
            .await
            .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;

        let url = format!(
            "mysql://root:{}@{}:{}/{}",
            shared.password, shared.host, shared.port, db_name
        );
        let pool = MySqlPoolOptions::new()
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
impl Analyzer for MySqlDatabaseAnalyzer {
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
                let name = col.type_info().name().to_lowercase();
                map_string_type(&name)
            };
            columns.push(ColumnInfo {
                name,
                data_type,
                nullability: true,
            });
        }
        Ok(ResultSet { columns })
    }

    async fn get_all_tables(&self) -> Result<Vec<Table>> {
        let query = r#"
            SELECT 
                CAST(c.TABLE_NAME AS CHAR) as TABLE_NAME, 
                CAST(c.COLUMN_NAME AS CHAR) as COLUMN_NAME, 
                CAST(c.DATA_TYPE AS CHAR) as DATA_TYPE, 
                CAST(c.IS_NULLABLE AS CHAR) as IS_NULLABLE
            FROM information_schema.COLUMNS c
            JOIN information_schema.TABLES t ON c.TABLE_NAME = t.TABLE_NAME AND c.TABLE_SCHEMA = t.TABLE_SCHEMA
            WHERE c.TABLE_SCHEMA = DATABASE()
            AND t.TABLE_TYPE = 'BASE TABLE'
            ORDER BY c.TABLE_NAME, c.ORDINAL_POSITION
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
                .try_get("TABLE_NAME")
                .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;
            let column_name: String = row
                .try_get("COLUMN_NAME")
                .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;
            let data_type_str: String = row
                .try_get("DATA_TYPE")
                .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;
            let is_nullable: String = row
                .try_get("IS_NULLABLE")
                .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;

            let data_type = map_string_type(&data_type_str);

            // MySQL IS_NULLABLE is "YES" or "NO"
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
                    "Missing table entry while collecting MySQL metadata: {}",
                    table_name
                )));
            }
        }

        let mut tables = Vec::with_capacity(table_order.len());
        for name in table_order {
            let columns = tables_map.remove(&name).ok_or_else(|| {
                AnalyzerError::AnalysisError(format!(
                    "Missing collected columns for MySQL table: {}",
                    name
                ))
            })?;
            tables.push(Table { columns, name });
        }

        Ok(tables)
    }
}

impl Drop for MySqlDatabaseAnalyzer {
    fn drop(&mut self) {
        let db_name = self.db_name.clone();
        let admin_url = self.admin_url.clone();

        // Try to drop the database in a blocking manner
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                if let Ok(admin_pool) = MySqlPoolOptions::new().connect(&admin_url).await {
                    let _ = admin_pool
                        .execute(format!("DROP DATABASE IF EXISTS {}", db_name).as_str())
                        .await;
                }
            });
        }
    }
}

fn map_string_type(t: &str) -> DataType {
    let original = t.to_lowercase();
    let unsigned = original.contains("unsigned");
    let base = original.replace("unsigned", "");
    let base = base.trim();

    match base {
        "boolean" | "bool" => DataType::Bool,
        "tinyint" => {
            if unsigned {
                DataType::TinyInt(true)
            } else {
                DataType::Bool
            }
        },
        "smallint" => DataType::SmallInt(unsigned),
        "int" | "integer" | "mediumint" => DataType::Int(unsigned),
        "bigint" => DataType::BigInt(unsigned),
        "float" => DataType::Float,
        "double" => DataType::Double,
        "decimal" | "numeric" => DataType::Decimal,
        "varchar" | "char" | "text" | "longtext" | "mediumtext" => DataType::Text,
        "date" => DataType::Date,
        "datetime" => DataType::DateTime,
        "timestamp" => DataType::Timestamp,
        "json" => DataType::Json,
        "blob" | "binary" | "varbinary" | "longblob" => DataType::Binary,
        _ => DataType::Custom(original),
    }
}
