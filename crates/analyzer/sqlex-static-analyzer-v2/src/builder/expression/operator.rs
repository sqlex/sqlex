use sqlex_analyzer::error::AnalyzerError;
use sqlparser::ast::{self, BinaryOperator, Expr, UnaryOperator};

use crate::{builder::RelationBuilder, node::expression::Expression};

#[allow(dead_code)]
impl RelationBuilder<'_> {
    pub(super) fn build_unary_operator_expression(
        &mut self,
        op: &UnaryOperator,
        expr: &Expr,
    ) -> Result<Expression, AnalyzerError> {
        let _ = (op, expr);
        todo!("unary expression builder is not implemented yet")
    }

    pub(super) fn build_binary_operator_expression(
        &mut self,
        left: &Expr,
        op: &BinaryOperator,
        right: &Expr,
    ) -> Result<Expression, AnalyzerError> {
        let _ = (left, op, right);
        todo!("binary expression builder is not implemented yet")
    }

    pub(super) fn build_is_null_expression(
        &mut self,
        expr: &Expr,
        negated: bool,
    ) -> Result<Expression, AnalyzerError> {
        let _ = (expr, negated);
        todo!("IS NULL expression builder is not implemented yet")
    }

    pub(super) fn build_in_list_expression(
        &mut self,
        expr: &Expr,
        list: &[Expr],
        negated: bool,
    ) -> Result<Expression, AnalyzerError> {
        let _ = (expr, list, negated);
        todo!("IN list expression builder is not implemented yet")
    }

    pub(super) fn build_case_expression(
        &mut self,
        operand: Option<&Expr>,
        conditions: &[Expr],
        results: &[Expr],
        else_expr: Option<&Expr>,
    ) -> Result<Expression, AnalyzerError> {
        let _ = (operand, conditions, results, else_expr);
        todo!("CASE expression builder is not implemented yet")
    }

    pub(super) fn build_cast_expression(
        &mut self,
        expr: &Expr,
        data_type: &ast::DataType,
    ) -> Result<Expression, AnalyzerError> {
        let _ = (expr, data_type);
        todo!("CAST expression builder is not implemented yet")
    }
}
