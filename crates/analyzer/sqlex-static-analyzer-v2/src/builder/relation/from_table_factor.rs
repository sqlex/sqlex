use sqlex_analyzer::{error::AnalyzerError, extension::object_name_ext::ObjectNameExt};
use sqlparser::ast;

use crate::{arena::RelationId, builder::RelationBuilder, node::relation::scan::ScanRelation};

#[allow(dead_code)]
impl RelationBuilder<'_> {
    pub(crate) fn build_table_factor(
        &mut self,
        table_factor: ast::TableFactor,
    ) -> Result<RelationId, AnalyzerError> {
        match table_factor {
            ast::TableFactor::Table { name, .. } => self.build_table_scan(name),
            ast::TableFactor::Derived { .. } => Err(AnalyzerError::todo(
                "derived table builder is not implemented yet",
            )),
            _ => Err(AnalyzerError::todo(
                "table factor builder is not implemented yet",
            )),
        }
    }

    fn build_table_scan(&mut self, name: ast::ObjectName) -> Result<RelationId, AnalyzerError> {
        let normalized_table_name = name.to_normalized_string(self.dialect);
        let _ = self.catalog.get_table(&normalized_table_name)?;

        Ok(self.arena.insert(ScanRelation {
            table: normalized_table_name,
        })?)
    }
}
