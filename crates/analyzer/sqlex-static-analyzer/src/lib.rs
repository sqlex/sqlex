//! Static SQL Analyzer
//!
//! A static SQL analyzer that infers result set types and nullability
//! without requiring a database connection.
//!
//! # Architecture
//!
//! - [`schema`]: Database schema representation (tables, columns, constraints)
//! - [`plan`]: Query plan node tree for representing SQL queries
//! - [`ddl`]: DDL statement parser for building schema
//! - [`analyzer`]: Query analyzer for building plan trees
//! - [`nullability`]: Nullability inference rules
//! - [`types`]: Type inference rules

pub mod analyzer;
pub mod ddl;
pub mod nullability;
pub mod plan;
pub mod schema;
pub mod types;

// Re-exports
pub use analyzer::QueryAnalyzer;
use async_trait::async_trait;
pub use plan::{JoinKind, PlanNode, TypedExpr};
pub use schema::{ColumnDef, Dialect, ForeignKeyDef, Schema, TableDef};
use sqlex_analyzer::{Analyzer, AnalyzerError, Result, ResultSet, Table};

/// Static SQL analyzer implementation
pub struct StaticAnalyzer {
    schema: Schema,
}

impl Default for StaticAnalyzer {
    fn default() -> Self {
        Self::new(Dialect::PostgreSQL)
    }
}

impl StaticAnalyzer {
    /// Create a new static analyzer with the given dialect
    pub fn new(dialect: Dialect) -> Self {
        Self {
            schema: Schema::new(dialect),
        }
    }

    /// Get a reference to the schema
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Get a mutable reference to the schema
    pub fn schema_mut(&mut self) -> &mut Schema {
        &mut self.schema
    }
}

#[async_trait]
impl Analyzer for StaticAnalyzer {
    async fn execute(&mut self, sql: &str) -> Result<()> {
        self.schema
            .execute_ddl(sql)
            .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))
    }

    async fn analyze(&self, sql: &str) -> Result<ResultSet> {
        let mut analyzer = QueryAnalyzer::new(&self.schema);
        analyzer.analyze(sql)
    }

    async fn get_all_tables(&self) -> Result<Vec<Table>> {
        let tables = self
            .schema
            .tables
            .values()
            .map(|t| Table {
                name: t.name.clone(),
                columns: t
                    .columns
                    .iter()
                    .map(|c| sqlex_common::ColumnInfo {
                        name: c.name.clone(),
                        data_type: c.data_type.clone(),
                        nullability: c.nullable,
                    })
                    .collect(),
            })
            .collect();
        Ok(tables)
    }
}
