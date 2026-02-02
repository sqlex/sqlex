use async_trait::async_trait;
use sqlex_analyzer::{Analyzer, AnalyzerError, ColumnInfo, DataType, Result, ResultSet, Table};
use sqlx::{
    Column, Executor, Row, Statement, TypeInfo,
    mysql::{MySqlPool, MySqlPoolOptions, MySqlTypeInfo},
};
use testcontainers::{ContainerAsync, GenericImage, ImageExt, runners::AsyncRunner};

use crate::utils::parse_retry_connect;

pub struct MySqlDatabaseAnalyzer {
    pool: MySqlPool,
    _container: ContainerAsync<GenericImage>,
}

impl MySqlDatabaseAnalyzer {
    pub async fn new() -> Result<Self> {
        let image = GenericImage::new("mysql", "8")
            .with_env_var("MYSQL_ROOT_PASSWORD", "mysql")
            .with_env_var("MYSQL_DATABASE", "sqlex_test");

        let container = image.start().await.map_err(|e| {
            AnalyzerError::ExecutionError(format!("Failed to start mysql container: {}", e))
        })?;

        let host = container
            .get_host()
            .await
            .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;
        let port = container
            .get_host_port_ipv4(3306)
            .await
            .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;
        let url = format!("mysql://root:mysql@{}:{}/sqlex_test", host, port);

        let pool = parse_retry_connect(|| MySqlPoolOptions::new().connect(&url)).await?;

        Ok(Self {
            pool,
            _container: container,
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
            let data_type = map_type(col.type_info());
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
            tables_map.get_mut(&table_name).unwrap().push(col_info);
        }

        let tables = table_order
            .into_iter()
            .map(|name| Table {
                columns: tables_map.remove(&name).unwrap(),
                name,
            })
            .collect();

        Ok(tables)
    }
}

fn map_type(info: &MySqlTypeInfo) -> DataType {
    let name = info.name().to_lowercase();
    map_string_type(&name)
}

fn map_string_type(t: &str) -> DataType {
    match t.to_lowercase().as_str() {
        "boolean" | "bool" | "tinyint" => DataType::Bool,
        "smallint" => DataType::SmallInt,
        "int" | "integer" | "mediumint" => DataType::Int,
        "bigint" => DataType::BigInt,
        "float" => DataType::Float,
        "double" => DataType::Double,
        "decimal" | "numeric" => DataType::Decimal,
        "varchar" | "char" | "text" | "longtext" | "mediumtext" => DataType::Text,
        "date" => DataType::Date,
        "datetime" => DataType::DateTime,
        "timestamp" => DataType::Timestamp,
        "json" => DataType::Json,
        "blob" | "binary" | "varbinary" | "longblob" => DataType::Binary,
        other => DataType::Custom(other.to_string()),
    }
}
