use sqlex_analyzer::error::AnalyzerError;
use sqlparser::ast;

use crate::{arena::RelationId, builder::RelationBuilder};

#[allow(dead_code)]
impl RelationBuilder<'_> {
    pub(crate) fn build_select(
        &mut self,
        select: ast::Select,
    ) -> Result<RelationId, AnalyzerError> {
        let relation = self.build_select_from(&select)?;
        let relation = self.build_select_filters(relation, &select)?;
        self.build_select_projection(relation, &select)
    }

    fn build_select_from(&mut self, select: &ast::Select) -> Result<RelationId, AnalyzerError> {
        if select.from.is_empty() {
            todo!("SELECT without FROM builder is not implemented yet")
        }

        if select.from.len() != 1 {
            return Err(AnalyzerError::todo(
                "multiple FROM items are not implemented yet",
            ));
        }

        self.build_table_with_joins(select.from[0].clone())
    }

    fn build_select_filters(
        &mut self,
        input: RelationId,
        select: &ast::Select,
    ) -> Result<RelationId, AnalyzerError> {
        if let Some(selection) = &select.selection {
            let _ = self.build_expression(selection)?;
        }
        if let Some(having) = &select.having {
            let _ = self.build_expression(having)?;
        }

        let _ = input;
        todo!("select WHERE/GROUP/HAVING builder is not implemented yet")
    }

    fn build_select_projection(
        &mut self,
        input: RelationId,
        select: &ast::Select,
    ) -> Result<RelationId, AnalyzerError> {
        for item in &select.projection {
            match item {
                ast::SelectItem::UnnamedExpr(expr) => {
                    let _ = self.build_expression(expr)?;
                },
                ast::SelectItem::ExprWithAlias { expr, .. } => {
                    let _ = self.build_expression(expr)?;
                },
                ast::SelectItem::Wildcard(_) | ast::SelectItem::QualifiedWildcard(_, _) => {},
            }
        }

        let _ = input;
        todo!("select projection builder is not implemented yet")
    }
}
