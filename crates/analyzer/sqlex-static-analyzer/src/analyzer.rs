use async_trait::async_trait;
use sqlex_analyzer::{Analyzer, Result};
use sqlex_common::types::{ResultSet, Table};

use crate::{
    StaticAnalyzer, algebraizer::Algebraizer, catalog::mutator::CatalogMutator,
    diagnostics::Diagnostic, infer::Inferencer, parser,
};

#[async_trait]
impl Analyzer for StaticAnalyzer {
    async fn execute(&mut self, sql: &str) -> Result<()> {
        let statements = parser::parse_statements(self.dialect, sql)
            .map_err(Diagnostic::into_execution_error)?;
        let mutator = CatalogMutator::new(self.dialect);

        for statement in &statements {
            mutator
                .apply_statement(&mut self.catalog, statement)
                .map_err(Diagnostic::into_execution_error)?;
        }

        Ok(())
    }

    async fn analyze(&self, sql: &str) -> Result<ResultSet> {
        let query_statement = parser::parse_query_statement(self.dialect, sql)
            .map_err(Diagnostic::into_analysis_error)?;
        let algebraizer = Algebraizer::new(self.dialect, &self.catalog, &self.functions);
        let relation = algebraizer
            .build(&query_statement)
            .map_err(Diagnostic::into_analysis_error)?;
        let inferencer = Inferencer::new(self.dialect, &self.catalog, &self.functions);
        let metadata = inferencer
            .infer(&relation)
            .map_err(Diagnostic::into_analysis_error)?;

        Ok(metadata.to_result_set())
    }

    async fn get_all_tables(&self) -> Result<Vec<Table>> {
        Ok(self.catalog.to_tables())
    }
}
