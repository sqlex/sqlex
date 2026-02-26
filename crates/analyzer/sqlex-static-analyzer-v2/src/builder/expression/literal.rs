use sqlex_analyzer::error::AnalyzerError;
use sqlparser::ast::Value;

use crate::{builder::RelationBuilder, node::expression::Expression};

#[allow(dead_code)]
impl RelationBuilder<'_> {
    pub(super) fn build_literal_expression(
        &mut self,
        value: &Value,
    ) -> Result<Expression, AnalyzerError> {
        let _ = value;
        todo!("literal expression builder is not implemented yet")
    }
}
