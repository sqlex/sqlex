use sqlex_analyzer::error::AnalyzerError;
use sqlparser::ast;

use crate::{arena::RelationId, builder::RelationBuilder};

#[allow(dead_code)]
impl RelationBuilder<'_> {
    pub(crate) fn build_table_with_joins(
        &mut self,
        table_with_joins: ast::TableWithJoins,
    ) -> Result<RelationId, AnalyzerError> {
        self.build_join_chain(table_with_joins.relation, table_with_joins.joins)
    }

    fn build_join_chain(
        &mut self,
        left_relation: ast::TableFactor,
        joins: Vec<ast::Join>,
    ) -> Result<RelationId, AnalyzerError> {
        let mut relation = self.build_table_factor(left_relation)?;
        for join in joins {
            relation = self.build_join_relation(relation, join)?;
        }
        Ok(relation)
    }
}
