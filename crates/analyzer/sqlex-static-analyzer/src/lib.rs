//! Static SQL Analyzer
//!
//! A static SQL analyzer that infers result set types and nullability
//! without requiring a database connection.

use async_trait::async_trait;
use sqlex_analyzer::{Analyzer, Result};
use sqlex_common::{
    dialect::Dialect,
    types::{ResultSet, Table},
};

/// Static SQL analyzer implementation
pub struct StaticAnalyzer {
    _dialect: Dialect,
}

impl StaticAnalyzer {
    /// Create a new static analyzer with the given dialect
    pub fn new(dialect: Dialect) -> Self {
        Self { _dialect: dialect }
    }
}

#[async_trait]
impl Analyzer for StaticAnalyzer {
    async fn execute(&mut self, _sql: &str) -> Result<()> {
        todo!()
    }

    async fn analyze(&self, _sql: &str) -> Result<ResultSet> {
        todo!()
    }

    async fn get_all_tables(&self) -> Result<Vec<Table>> {
        todo!()
    }
}
