//! Static SQL Analyzer
//!
//! A static SQL analyzer that infers result set types and nullability
//! without requiring a database connection.

use async_trait::async_trait;
use sqlex_analyzer::{Analyzer, AnalyzerError, Result};
use sqlex_common::{
    dialect::Dialect,
    types::{ResultSet, Table},
};

use crate::catalog::Catalog;

pub mod catalog;

/// Static SQL analyzer implementation
pub struct StaticAnalyzer {
    catalog: Catalog,
    dialect: Dialect,
}

impl StaticAnalyzer {
    /// Create a new static analyzer with the given dialect
    pub fn new(dialect: Dialect) -> Self {
        Self {
            catalog: Catalog::new(dialect),
            dialect,
        }
    }
}

#[async_trait]
impl Analyzer for StaticAnalyzer {
    async fn execute(&mut self, sql: &str) -> Result<()> {
        self.catalog
            .apply_ddl(sql)
            .map_err(|e| AnalyzerError::ExecutionError(e.to_string()))
    }

    async fn analyze(&self, sql: &str) -> Result<ResultSet> {
        todo!()
    }

    async fn get_all_tables(&self) -> Result<Vec<Table>> {
        let tables = self
            .catalog
            .tables
            .values()
            .map(|t| Table {
                name: t.name.clone(),
                columns: t
                    .columns
                    .iter()
                    .map(|c| sqlex_common::types::ColumnInfo {
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
