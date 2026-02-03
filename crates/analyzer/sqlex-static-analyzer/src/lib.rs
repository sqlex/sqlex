//! Static SQL Analyzer
//!
//! A static SQL analyzer that infers result set types and nullability
//! without requiring a database connection.

pub mod planner;
pub mod schema;

// Re-exports
use async_trait::async_trait;
pub use planner::{BuildContext, LogicalNode, PlanNode, PlanNodeColumn, nodes::join::JoinKind};
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
        let ctx = BuildContext::new(&self.schema);
        let plan = ctx.build(sql)?;
        let columns = plan
            .columns()
            .iter()
            .map(|c| sqlex_common::ColumnInfo {
                name: c.name.clone(),
                data_type: c.data_type.clone(),
                nullability: c.nullability,
            })
            .collect();
        Ok(ResultSet { columns })
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
