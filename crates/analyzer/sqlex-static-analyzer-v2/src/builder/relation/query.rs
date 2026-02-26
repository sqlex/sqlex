use sqlex_analyzer::error::AnalyzerError;
use sqlparser::ast;

use crate::{arena::RelationId, builder::RelationBuilder};

impl RelationBuilder<'_> {
    pub(crate) fn build_query(&mut self, query: ast::Query) -> Result<RelationId, AnalyzerError> {
        let has_order_by = query.order_by.is_some();
        let has_limit = query.limit.is_some();
        let has_offset = query.offset.is_some();

        let relation = self.build_query_body(*query.body)?;
        let relation = self.apply_query_order_by(relation, has_order_by)?;
        self.apply_query_limit_offset(relation, has_limit, has_offset)
    }

    fn apply_query_order_by(
        &mut self,
        input: RelationId,
        has_order_by: bool,
    ) -> Result<RelationId, AnalyzerError> {
        if !has_order_by {
            return Ok(input);
        }

        let _ = (input, has_order_by);
        todo!("query ORDER BY builder is not implemented yet")
    }

    fn apply_query_limit_offset(
        &mut self,
        input: RelationId,
        has_limit: bool,
        has_offset: bool,
    ) -> Result<RelationId, AnalyzerError> {
        if !has_limit && !has_offset {
            return Ok(input);
        }

        let _ = (input, has_limit, has_offset);
        todo!("query LIMIT/OFFSET builder is not implemented yet")
    }
}
