use sqlex_analyzer::error::AnalyzerError;
use sqlparser::ast::Expr;

use crate::{builder::RelationBuilder, node::expression::Expression};

mod column;
mod function;
mod literal;
mod operator;
mod subquery;

#[allow(dead_code)]
impl RelationBuilder<'_> {
    pub(crate) fn build_expression(&mut self, expr: &Expr) -> Result<Expression, AnalyzerError> {
        match expr {
            Expr::Identifier(identifier) => self.build_identifier_expression(identifier),
            Expr::CompoundIdentifier(identifiers) => {
                self.build_compound_identifier_expression(identifiers)
            },
            Expr::Value(value) => self.build_literal_expression(value),
            Expr::Function(function) => self.build_function_expression(function),
            Expr::UnaryOp { op, expr } => self.build_unary_operator_expression(op, expr),
            Expr::BinaryOp { left, op, right } => {
                self.build_binary_operator_expression(left, op, right)
            },
            Expr::Nested(expr) => self.build_expression(expr),
            Expr::Subquery(query) => self.build_scalar_subquery_expression(query),
            Expr::Exists { subquery, negated } => {
                self.build_exists_subquery_expression(subquery, *negated)
            },
            Expr::InList {
                expr,
                list,
                negated,
            } => self.build_in_list_expression(expr, list, *negated),
            Expr::InSubquery {
                expr,
                subquery,
                negated,
            } => self.build_in_subquery_expression(expr, subquery, *negated),
            Expr::IsNull(expr) => self.build_is_null_expression(expr, false),
            Expr::IsNotNull(expr) => self.build_is_null_expression(expr, true),
            Expr::Case {
                operand,
                conditions,
                results,
                else_result,
            } => self.build_case_expression(
                operand.as_deref(),
                conditions,
                results,
                else_result.as_deref(),
            ),
            Expr::Cast {
                expr, data_type, ..
            } => self.build_cast_expression(expr, data_type),
            _ => Err(AnalyzerError::todo(
                "scalar expression builder is not implemented yet",
            )),
        }
    }
}
