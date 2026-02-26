use sqlex_analyzer::error::AnalyzerError;
use sqlparser::ast;

use crate::{arena::RelationId, builder::RelationBuilder};

#[allow(dead_code)]
impl RelationBuilder<'_> {
    pub(super) fn build_join_relation(
        &mut self,
        left: RelationId,
        join: ast::Join,
    ) -> Result<RelationId, AnalyzerError> {
        let right = self.build_table_factor(join.relation)?;
        self.build_join_from_parts(left, right, join.join_operator)
    }

    fn build_join_from_parts(
        &mut self,
        left: RelationId,
        right: RelationId,
        join_operator: ast::JoinOperator,
    ) -> Result<RelationId, AnalyzerError> {
        let _ = (left, right, join_operator);
        todo!("join relation builder is not implemented yet")
    }
}
