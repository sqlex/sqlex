use std::collections::HashMap;

use sqlex_analyzer::{AnalyzerError, Result};
use sqlex_common::DataType;

/// Resolved column information (internal)
#[derive(Debug, Clone)]
pub struct ResolvedColumn {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub source_alias: Option<String>,
}

/// Scope for column resolution
#[derive(Debug, Default, Clone)]
pub struct Scope {
    /// Available tables/aliases and their columns
    pub tables: HashMap<String, Vec<ResolvedColumn>>,
}

impl Scope {
    /// Resolve a column reference
    pub fn resolve_column(
        &self,
        table_alias: Option<&str>,
        col_name: &str,
    ) -> Result<ResolvedColumn> {
        if let Some(alias) = table_alias {
            if let Some(cols) = self.tables.get(alias) {
                if let Some(col) = cols.iter().find(|c| c.name == col_name) {
                    return Ok(col.clone());
                }
            }
            Err(AnalyzerError::AnalysisError(format!(
                "Column {}.{} not found",
                alias, col_name
            )))
        } else {
            let mut found = None;
            for cols in self.tables.values() {
                if let Some(col) = cols.iter().find(|c| c.name == col_name) {
                    if found.is_some() {
                        return Err(AnalyzerError::AnalysisError(format!(
                            "Ambiguous column {}",
                            col_name
                        )));
                    }
                    found = Some(col.clone());
                }
            }
            found.ok_or_else(|| {
                AnalyzerError::AnalysisError(format!("Column {} not found", col_name))
            })
        }
    }

    /// Add a table to the scope
    pub fn add_table(&mut self, alias: String, columns: Vec<ResolvedColumn>) {
        self.tables.insert(alias, columns);
    }

    pub fn merge(&mut self, other: Scope) {
        for (k, v) in other.tables {
            self.tables.insert(k, v);
        }
    }
}
