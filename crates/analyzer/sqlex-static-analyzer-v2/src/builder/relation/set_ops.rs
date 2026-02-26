use sqlex_analyzer::error::AnalyzerError;
use sqlparser::ast;

use crate::{arena::RelationId, builder::RelationBuilder};

impl RelationBuilder<'_> {
    pub(crate) fn build_query_body(
        &mut self,
        set_expr: ast::SetExpr,
    ) -> Result<RelationId, AnalyzerError> {
        self.build_set_expr(set_expr)
    }

    pub(crate) fn build_set_expr(
        &mut self,
        set_expr: ast::SetExpr,
    ) -> Result<RelationId, AnalyzerError> {
        match set_expr {
            ast::SetExpr::Select(select) => self.build_select(*select),
            ast::SetExpr::Query(query) => self.build_query(*query),
            ast::SetExpr::SetOperation {
                left,
                op,
                set_quantifier,
                right,
            } => self.build_set_operation(*left, op, set_quantifier, *right),
            _ => Err(AnalyzerError::todo(
                "set expression builder is not implemented yet",
            )),
        }
    }

    fn build_set_operation(
        &mut self,
        left: ast::SetExpr,
        op: ast::SetOperator,
        set_quantifier: ast::SetQuantifier,
        right: ast::SetExpr,
    ) -> Result<RelationId, AnalyzerError> {
        let _ = (left, op, set_quantifier, right);
        todo!("set operation relation builder is not implemented yet")
    }
}
