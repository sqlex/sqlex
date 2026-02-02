use async_trait::async_trait;
use rand::{Rng, distributions::Alphanumeric};
use sqlex_analyzer::{Analyzer, AnalyzerError, ColumnInfo, DataType, Result, ResultSet, Table};
use sqlx::{
    Column, Executor, Row, Statement, TypeInfo,
    postgres::{PgPool, PgPoolOptions, PgTypeInfo},
};
use testcontainers::{ContainerAsync, GenericImage, ImageExt, runners::AsyncRunner};

use crate::utils::parse_retry_connect;

pub struct PostgresDatabaseAnalyzer {
    pool: PgPool,
    _container: ContainerAsync<GenericImage>,
}

impl PostgresDatabaseAnalyzer {
    pub async fn new() -> Result<Self> {
        // Use Alphanumeric to ensure password characters are safe for the connection URL
        // without requiring percent-encoding.
        let password: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();

        let image = GenericImage::new("postgres", "16-alpine")
            .with_env_var("POSTGRES_PASSWORD", &password)
            .with_env_var("POSTGRES_DB", "sqlex");

        let container = image.start().await.map_err(|e| {
            AnalyzerError::ExecutionError(format!("Failed to start postgres container: {}", e))
        })?;

        let host = container
            .get_host()
            .await
            .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;
        let port = container
            .get_host_port_ipv4(5432)
            .await
            .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;
        let url = format!("postgres://postgres:{}@{}:{}/sqlex", password, host, port);

        let pool = parse_retry_connect(|| PgPoolOptions::new().connect(&url)).await?;

        Ok(Self {
            pool,
            _container: container,
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
            let type_info = col.type_info();
            let data_type = map_type(type_info);
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

fn map_type(info: &PgTypeInfo) -> DataType {
    let name = info.name().to_lowercase();
    map_udt(&name)
}

fn map_udt(udt: &str) -> DataType {
    match udt {
        "bool" | "boolean" => DataType::Bool,
        "int2" | "smallint" => DataType::SmallInt,
        "int4" | "integer" | "int" => DataType::Int,
        "int8" | "bigint" => DataType::BigInt,
        "float4" | "real" => DataType::Float,
        "float8" | "double precision" => DataType::Double,
        "numeric" | "decimal" => DataType::Decimal,
        "varchar" | "char" | "text" | "bpchar" => DataType::Text,
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
