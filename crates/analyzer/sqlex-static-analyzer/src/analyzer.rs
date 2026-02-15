use async_trait::async_trait;
use sqlex_analyzer::{Analyzer, Result};
use sqlex_common::types::{ResultSet, Table};

use crate::{
    StaticAnalyzer, algebra::planner::Algebraizer, catalog::mutator::CatalogMutator,
    diagnostics::Diagnostic, infer::engine::InferEngine, parser,
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
        let planner = Algebraizer::new(self.dialect);
        let relational_expr = planner
            .build(&query_statement, &self.catalog, &self.functions)
            .map_err(Diagnostic::into_analysis_error)?;
        let infer_engine = InferEngine::new(self.dialect, &self.functions);
        let metadata = infer_engine
            .infer(&relational_expr, &self.catalog)
            .map_err(Diagnostic::into_analysis_error)?;

        Ok(metadata.to_result_set())
    }

    async fn get_all_tables(&self) -> Result<Vec<Table>> {
        Ok(self.catalog.to_tables())
    }
}
