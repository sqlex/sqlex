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

use crate::{
    algebraize::Algebraizer, catalog::Catalog, diagnostics::DiagnosticSeverity, infer::Inferrer,
};

mod algebraize;
pub mod catalog;
mod diagnostics;
mod functions;
mod infer;
pub mod ir;
mod keywords;

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
        // Phase 2: Algebraize — AST + Catalog → RelationalExpr
        let alg_result = Algebraizer::new(self.dialect, &self.catalog).algebraize(sql);

        if alg_result
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error)
        {
            let message = alg_result
                .diagnostics
                .iter()
                .filter(|d| d.severity == DiagnosticSeverity::Error)
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(AnalyzerError::AnalysisError(message));
        }

        let expr = alg_result.expr.ok_or_else(|| {
            AnalyzerError::AnalysisError("Algebraize produced no expression".to_string())
        })?;

        // Phase 3: Infer — RelationalExpr → OutputSchema
        let infer_result = Inferrer::new(self.dialect, &self.catalog).infer(&expr);

        // Check for inference errors (e.g. ambiguous columns)
        if infer_result
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error)
        {
            let message = infer_result
                .diagnostics
                .iter()
                .filter(|d| d.severity == DiagnosticSeverity::Error)
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(AnalyzerError::AnalysisError(message));
        }

        let output = infer_result.output.ok_or_else(|| {
            AnalyzerError::AnalysisError("Inference produced no output schema".to_string())
        })?;

        let columns = output
            .columns
            .into_iter()
            .map(|c| sqlex_common::types::ColumnInfo {
                name: c.name,
                data_type: c.data_type,
                nullability: c.nullability,
            })
            .collect();

        Ok(ResultSet {
            columns,
            cardinality: output.cardinality,
        })
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
