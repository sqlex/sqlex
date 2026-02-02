use async_trait::async_trait;
use sqlex_analyzer::{Analyzer, AnalyzerError, ColumnInfo, DataType, Result, ResultSet};
use sqlex_common::DatabaseType;
use sqlx::{
    Column, Executor, Statement, TypeInfo,
    mysql::{MySqlPool, MySqlPoolOptions, MySqlTypeInfo},
    postgres::{PgPool, PgPoolOptions, PgTypeInfo},
    sqlite::{SqlitePool, SqlitePoolOptions, SqliteTypeInfo},
};

pub struct DatabaseAnalyzer {
    pool: AnyInternalPool,
}

enum AnyInternalPool {
    Postgres(PgPool),
    MySQL(MySqlPool),
    SQLite(SqlitePool),
}

impl DatabaseAnalyzer {
    pub async fn new(url: &str, db_type: DatabaseType) -> Result<Self> {
        let pool = match db_type {
            DatabaseType::Postgres => {
                let p = PgPoolOptions::new()
                    .connect(url)
                    .await
                    .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;
                AnyInternalPool::Postgres(p)
            },
            DatabaseType::MySQL => {
                let p = MySqlPoolOptions::new()
                    .connect(url)
                    .await
                    .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;
                AnyInternalPool::MySQL(p)
            },
            DatabaseType::SQLite => {
                let p = SqlitePoolOptions::new()
                    .connect(url)
                    .await
                    .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;
                AnyInternalPool::SQLite(p)
            },
        };
        Ok(Self { pool })
    }
}

#[async_trait]
impl Analyzer for DatabaseAnalyzer {
    async fn execute(&mut self, sql: &str) -> Result<()> {
        match &self.pool {
            AnyInternalPool::Postgres(p) => {
                p.execute(sql)
                    .await
                    .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;
            },
            AnyInternalPool::MySQL(p) => {
                p.execute(sql)
                    .await
                    .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;
            },
            AnyInternalPool::SQLite(p) => {
                p.execute(sql)
                    .await
                    .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))?;
            },
        }
        Ok(())
    }

    async fn analyze(&self, sql: &str) -> Result<ResultSet> {
        match &self.pool {
            AnyInternalPool::Postgres(p) => analyze_postgres(p, sql).await,
            AnyInternalPool::MySQL(p) => analyze_mysql(p, sql).await,
            AnyInternalPool::SQLite(p) => analyze_sqlite(p, sql).await,
        }
    }

    async fn get_all_tables(&self) -> Result<Vec<sqlex_analyzer::Table>> {
        todo!()
    }
}

async fn analyze_postgres(pool: &PgPool, sql: &str) -> Result<ResultSet> {
    let stmt = pool
        .prepare(sql)
        .await
        .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;

    let mut columns = Vec::new();
    for col in stmt.columns() {
        let name = col.name().to_string();
        let type_info = col.type_info();
        let data_type = map_postgres_type(type_info);
        // Postgres Describe doesn't always guarantee nullability correct in all drivers,
        // but sqlx might have it.
        // `col.ordinal()` etc.
        // Usually sqlx::Column doesn't expose nullability directly for PREPARE results easily in all versions.
        // But let's assume unknown or true for now if not available.
        // Actually sqlx `Column` trait doesn't have `nullable()`. `Describe` struct has it.
        // But `pool.prepare` returns `Statement` which has `columns()`.

        columns.push(ColumnInfo {
            name,
            data_type,
            nullability: true, // Default to true, hard to get from Prepare without Describe logic sometimes
        });
    }

    Ok(ResultSet { columns })
}

fn map_postgres_type(info: &PgTypeInfo) -> DataType {
    let name = info.name().to_lowercase();
    match name.as_str() {
        "bool" | "boolean" => DataType::Bool,
        "int2" | "smallint" => DataType::SmallInt,
        "int4" | "integer" | "int" => DataType::Int,
        "int8" | "bigint" => DataType::BigInt,
        "float4" | "real" => DataType::Float,
        "float8" | "double precision" => DataType::Double,
        "numeric" | "decimal" => DataType::Decimal,
        "varchar" | "char" | "text" | "bpchar" => DataType::Text, // Simplification
        "date" => DataType::Date,
        "time" => DataType::Time,
        "timestamp" | "timestamptz" => DataType::Timestamp,
        "uuid" => DataType::Uuid,
        "json" | "jsonb" => DataType::Json,
        "bytea" => DataType::Binary,
        _ => DataType::Custom(name),
    }
}

async fn analyze_mysql(pool: &MySqlPool, sql: &str) -> Result<ResultSet> {
    let stmt = pool
        .prepare(sql)
        .await
        .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;
    let mut columns = Vec::new();
    for col in stmt.columns() {
        let name = col.name().to_string();
        let data_type = map_mysql_type(col.type_info());
        columns.push(ColumnInfo {
            name,
            data_type,
            nullability: true,
        });
    }
    Ok(ResultSet { columns })
}

fn map_mysql_type(info: &MySqlTypeInfo) -> DataType {
    let name = info.name().to_lowercase();
    match name.as_str() {
        "boolean" | "bool" | "tinyint" => DataType::Bool, // MySQL tinyint(1) is bool often
        "smallint" => DataType::SmallInt,
        "int" | "integer" | "mediumint" => DataType::Int,
        "bigint" => DataType::BigInt,
        "float" => DataType::Float,
        "double" => DataType::Double,
        "decimal" | "numeric" => DataType::Decimal,
        "varchar" | "char" | "text" => DataType::Text,
        "date" => DataType::Date,
        "datetime" => DataType::DateTime,
        "timestamp" => DataType::Timestamp,
        "json" => DataType::Json,
        "blob" | "binary" | "varbinary" => DataType::Binary,
        _ => DataType::Custom(name),
    }
}

async fn analyze_sqlite(pool: &SqlitePool, sql: &str) -> Result<ResultSet> {
    let stmt = pool
        .prepare(sql)
        .await
        .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;
    let mut columns = Vec::new();
    for col in stmt.columns() {
        let name = col.name().to_string();
        let data_type = map_sqlite_type(col.type_info());
        columns.push(ColumnInfo {
            name,
            data_type,
            nullability: true,
        });
    }
    Ok(ResultSet { columns })
}

fn map_sqlite_type(info: &SqliteTypeInfo) -> DataType {
    let name = info.name().to_lowercase();
    match name.as_str() {
        "integer" => DataType::BigInt, // Sqlite integer is i64
        "real" => DataType::Double,
        "text" => DataType::Text,
        "blob" => DataType::Binary,
        _ => DataType::Custom(name),
    }
}
