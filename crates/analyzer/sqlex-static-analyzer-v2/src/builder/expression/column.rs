use sqlex_analyzer::error::AnalyzerError;
use sqlparser::ast::Ident;

use crate::{
    builder::RelationBuilder,
    node::expression::{Expression, column_ref::ColumnRef},
};

#[allow(dead_code)]
impl RelationBuilder<'_> {
    pub(super) fn build_identifier_expression(
        &mut self,
        identifier: &Ident,
    ) -> Result<Expression, AnalyzerError> {
        Ok(Expression::from(ColumnRef::unresolved(vec![
            identifier.clone(),
        ])))
    }

    pub(super) fn build_compound_identifier_expression(
        &mut self,
        identifiers: &[Ident],
    ) -> Result<Expression, AnalyzerError> {
        if identifiers.is_empty() {
            return Err(AnalyzerError::analysis(
                "A0014",
                "compound identifier must contain at least one part",
            ));
        }
        Ok(Expression::from(ColumnRef::unresolved(
            identifiers.to_vec(),
        )))
    }
}
