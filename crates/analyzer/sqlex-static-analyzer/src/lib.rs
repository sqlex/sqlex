//! Static SQL Analyzer
//!
//! A static SQL analyzer that infers result set types and nullability
//! without requiring a database connection.

pub mod analysis;
pub mod catalog;
pub mod ir;

// Re-exports
use analysis::{AnalysisEngine, diagnostics::DiagnosticSeverity};
use async_trait::async_trait;
pub use catalog::{Catalog, ColumnDef, ForeignKeyDef, TableDef};
use sqlex_analyzer::{Analyzer, AnalyzerError, Result, ResultSet, Table};
pub use sqlex_common::Dialect;

/// Static SQL analyzer implementation
pub struct StaticAnalyzer {
    catalog: Catalog,
    analysis: AnalysisEngine,
}

impl StaticAnalyzer {
    /// Create a new static analyzer with the given dialect
    pub fn new(dialect: Dialect) -> Self {
        Self {
            catalog: Catalog::new(dialect),
            analysis: AnalysisEngine::new(dialect),
        }
    }

    /// Get a reference to the catalog
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// Get a mutable reference to the catalog
    pub fn catalog_mut(&mut self) -> &mut Catalog {
        &mut self.catalog
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
        let analysis = self.analysis.analyze(&self.catalog, sql);
        if analysis
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error)
        {
            let message = analysis
                .diagnostics
                .iter()
                .filter(|d| d.severity == DiagnosticSeverity::Error)
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(AnalyzerError::AnalysisError(message));
        }

        let output = analysis.output.ok_or_else(|| {
            AnalyzerError::AnalysisError("Analysis produced no output schema".to_string())
        })?;

        let columns = output
            .columns
            .into_iter()
            .map(|c| sqlex_common::ColumnInfo {
                name: c.name,
                data_type: c.data_type,
                nullability: c.nullability,
            })
            .collect();

        Ok(ResultSet { columns })
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
