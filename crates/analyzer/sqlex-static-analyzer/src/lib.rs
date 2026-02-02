use std::collections::HashMap;

use async_trait::async_trait;
use sqlex_analyzer::{Analyzer, ColumnInfo, Result, ResultSet};

#[derive(Default)]
pub struct StaticAnalyzer {
    // Map TableName -> Columns
    _tables: HashMap<String, Vec<ColumnInfo>>,
}

impl StaticAnalyzer {
    pub fn new() -> Self {
        Self {
            _tables: HashMap::new(),
        }
    }
}

#[async_trait]
impl Analyzer for StaticAnalyzer {
    async fn execute(&mut self, _sql: &str) -> Result<()> {
        // TODO: Implement DDL parsing and schema update
        Ok(())
    }

    async fn analyze(&self, _sql: &str) -> Result<ResultSet> {
        // TODO: Implement query analysis
        Ok(ResultSet { columns: vec![] })
    }

    async fn get_all_tables(&self) -> Result<Vec<sqlex_analyzer::Table>> {
        todo!()
    }
}
