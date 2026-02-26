use sqlex_analyzer::error::AnalyzerError;
use sqlparser::ast::{Expr, Query};

use crate::{builder::RelationBuilder, node::expression::Expression};

#[allow(dead_code)]
impl RelationBuilder<'_> {
    pub(super) fn build_scalar_subquery_expression(
        &mut self,
        query: &Query,
    ) -> Result<Expression, AnalyzerError> {
        let _ = query;
        todo!("scalar subquery expression builder is not implemented yet")
    }

    pub(super) fn build_exists_subquery_expression(
        &mut self,
        query: &Query,
        negated: bool,
    ) -> Result<Expression, AnalyzerError> {
        let _ = (query, negated);
        todo!("exists subquery expression builder is not implemented yet")
    }

    pub(super) fn build_in_subquery_expression(
        &mut self,
        expr: &Expr,
        query: &Query,
        negated: bool,
    ) -> Result<Expression, AnalyzerError> {
        let _ = (expr, query, negated);
        todo!("IN subquery expression builder is not implemented yet")
    }
}
