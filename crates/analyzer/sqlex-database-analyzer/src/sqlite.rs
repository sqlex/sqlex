use async_trait::async_trait;
use sqlex_analyzer::{Analyzer, AnalyzerError, Result};
use sqlex_common::types::{ColumnInfo, DataType, ResultSet, Table};
use sqlx::{
    Column, Executor, Row, Statement, TypeInfo,
    sqlite::{SqlitePool, SqlitePoolOptions},
};

pub struct SqliteDatabaseAnalyzer {
    pool: SqlitePool,
}

impl SqliteDatabaseAnalyzer {
    pub async fn new() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl Analyzer for SqliteDatabaseAnalyzer {
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
        Ok(ResultSet {
            columns,
            cardinality: sqlex_common::types::Cardinality::Unknown,
        })
    }

    async fn get_all_tables(&self) -> Result<Vec<Table>> {
        let tables_query =
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'";
        let rows = sqlx::query(tables_query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;

        let mut tables = Vec::new();
        for row in rows {
            let table_name: String = row
                .try_get("name")
                .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;

            let pragma_query = format!("PRAGMA table_info('{}')", table_name);

            let col_rows = sqlx::query(&pragma_query)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;

            let mut columns = Vec::new();
            for col_row in col_rows {
                let name: String = col_row
                    .try_get("name")
                    .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;
                let type_str: String = col_row
                    .try_get("type")
                    .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;
                let notnull: i32 = col_row
                    .try_get("notnull")
                    .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;

                let data_type = map_string_type(&type_str);
                let nullability = notnull == 0;

                columns.push(ColumnInfo {
                    name,
                    data_type,
                    nullability,
                });
            }

            tables.push(Table {
                name: table_name,
                columns,
            });
        }

        Ok(tables)
    }
}

fn map_string_type(t: &str) -> DataType {
    let t = t.to_lowercase();
    if t.contains("int") {
        DataType::BigInt(false)
    } else if t.starts_with("varchar") || t.starts_with("varying character") || t.starts_with("nvarchar") {
        DataType::Varchar
    } else if t.starts_with("char") || t.starts_with("nchar") || t.starts_with("native character") {
        DataType::Char
    } else if t.contains("clob") || t.contains("text") {
        DataType::Text
    } else if t.contains("blob") {
        DataType::Binary
    } else if t.contains("real") || t.contains("floa") || t.contains("doub") {
        DataType::Double
    } else if t.contains("dec") || t.contains("numeric") {
        DataType::Decimal
    } else if t.contains("bool") {
        DataType::Bool
    } else if t.contains("datetime") {
        DataType::DateTime
    } else if t.contains("timestamp") {
        DataType::Timestamp
    } else if t.contains("date") {
        DataType::Date
    } else if t.contains("time") {
        DataType::Time
    } else {
        DataType::Custom(t)
    }
}
