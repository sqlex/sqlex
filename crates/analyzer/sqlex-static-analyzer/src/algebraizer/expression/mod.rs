use sqlex_analyzer::{
    error::AnalyzerError,
    extension::{data_type_ext::DataTypeExt, ident_ext::IdentExt},
};
use sqlex_common::types::DataType;
use sqlparser::ast::{BinaryOperator, Expr};

use crate::algebraizer::{
    Algebraizer, error_code,
    model::expression::{BoundBinaryOp, BoundUnaryOp, Expression},
};

mod column;
mod function;
mod literal;
mod subquery;

impl Algebraizer<'_> {
    pub(crate) fn build_expression(
        &mut self,
        expr: &Expr,
    ) -> Result<(Expression, bool), AnalyzerError> {
        match expr {
            Expr::Identifier(ident) => {
                let normalized = ident.to_normalized_string(self.dialect);
                let binding = self.resolve_unqualified_column(&normalized)?;
                Ok((binding.into_scalar_expr(), false))
            },
            Expr::CompoundIdentifier(idents) => {
                if idents.is_empty() {
                    return Err(AnalyzerError::analysis(
                        error_code::EXPRESSION_EMPTY_COMPOUND_IDENTIFIER,
                        "empty compound identifier",
                    ));
                }
                let column_name = idents
                    .last()
                    .expect("not empty")
                    .to_normalized_string(self.dialect);
                let qualifier = idents[..idents.len() - 1]
                    .iter()
                    .map(|ident| ident.to_normalized_string(self.dialect))
                    .collect::<Vec<_>>()
                    .join(".");
                let binding = self.resolve_qualified_column(&qualifier, &column_name)?;
                Ok((binding.into_scalar_expr(), false))
            },
            Expr::Value(value) => Ok((
                Expression::Literal(self.build_literal_expression(value)?),
                false,
            )),
            Expr::Nested(inner) => self.build_expression(inner),
            Expr::UnaryOp { op, expr } => {
                let (inner, has_aggregate) = self.build_expression(expr)?;
                let op = match op {
                    sqlparser::ast::UnaryOperator::Plus => BoundUnaryOp::Pos,
                    sqlparser::ast::UnaryOperator::Minus => BoundUnaryOp::Neg,
                    sqlparser::ast::UnaryOperator::Not => BoundUnaryOp::Not,
                    _ => {
                        return Err(AnalyzerError::analysis(
                            error_code::UNARY_OPERATOR_UNSUPPORTED,
                            format!("unsupported unary operator in this iteration: {op}"),
                        ));
                    },
                };
                Ok((
                    Expression::UnaryOp {
                        op,
                        expr: Box::new(inner),
                    },
                    has_aggregate,
                ))
            },
            Expr::BinaryOp { left, op, right } => {
                let (left_expr, left_has_aggregate) = self.build_expression(left)?;
                let (right_expr, right_has_aggregate) = self.build_expression(right)?;
                let op = map_binary_operator(op)?;
                Ok((
                    Expression::BinaryOp {
                        left: Box::new(left_expr),
                        op,
                        right: Box::new(right_expr),
                    },
                    left_has_aggregate || right_has_aggregate,
                ))
            },
            Expr::Cast {
                expr, data_type, ..
            } => {
                let (inner, has_aggregate) = self.build_expression(expr)?;
                let target_type = DataType::from_sql_data_type(self.dialect, data_type);
                Ok((
                    Expression::Cast {
                        expr: Box::new(inner),
                        target_type,
                    },
                    has_aggregate,
                ))
            },
            Expr::Ceil { expr, field } => self.build_ceil_or_floor_expression("ceil", expr, field),
            Expr::Floor { expr, field } => {
                self.build_ceil_or_floor_expression("floor", expr, field)
            },
            Expr::Trim {
                expr,
                trim_where,
                trim_what,
                trim_characters,
            } => {
                if trim_where.is_some() || trim_what.is_some() || trim_characters.is_some() {
                    return Err(AnalyzerError::analysis(
                        error_code::TRIM_MODIFIERS_UNSUPPORTED,
                        "TRIM modifiers are not supported in this iteration",
                    ));
                }
                let (bound_expr, has_aggregate) = self.build_expression(expr)?;
                let bound_args = vec![bound_expr];
                let signature = self.functions.resolve_scalar("trim");
                self.validate_function_arity("trim", bound_args.len(), signature)?;
                Ok((
                    Expression::Function {
                        name: "trim".to_string(),
                        args: bound_args,
                    },
                    has_aggregate,
                ))
            },
            Expr::Function(function) => self.build_function_expression(function),
            Expr::IsNull(inner) => {
                let (bound, has_aggregate) = self.build_expression(inner)?;
                Ok((
                    Expression::IsNull {
                        expr: Box::new(bound),
                        negated: false,
                    },
                    has_aggregate,
                ))
            },
            Expr::IsNotNull(inner) => {
                let (bound, has_aggregate) = self.build_expression(inner)?;
                Ok((
                    Expression::IsNull {
                        expr: Box::new(bound),
                        negated: true,
                    },
                    has_aggregate,
                ))
            },
            Expr::Case {
                operand,
                conditions,
                results,
                else_result,
            } => {
                let mut has_aggregate = false;
                let mut when_clauses = Vec::new();

                for (condition, result) in conditions.iter().zip(results.iter()) {
                    let (bound_condition, condition_has_aggregate) =
                        self.build_expression(condition)?;
                    let (bound_result, result_has_aggregate) = self.build_expression(result)?;
                    has_aggregate |= condition_has_aggregate || result_has_aggregate;
                    when_clauses.push((bound_condition, bound_result));
                }

                let bound_operand = if let Some(operand) = operand {
                    let (bound, operand_has_aggregate) = self.build_expression(operand)?;
                    has_aggregate |= operand_has_aggregate;
                    Some(Box::new(bound))
                } else {
                    None
                };

                let bound_else = if let Some(else_expr) = else_result {
                    let (bound, else_has_aggregate) = self.build_expression(else_expr)?;
                    has_aggregate |= else_has_aggregate;
                    Some(Box::new(bound))
                } else {
                    None
                };

                Ok((
                    Expression::Case {
                        operand: bound_operand,
                        when_clauses,
                        else_expr: bound_else,
                    },
                    has_aggregate,
                ))
            },
            Expr::InList {
                expr,
                list,
                negated,
            } => {
                let (bound_expr, mut has_aggregate) = self.build_expression(expr)?;
                let mut bound_list = Vec::with_capacity(list.len());
                for item in list {
                    let (bound_item, item_has_aggregate) = self.build_expression(item)?;
                    has_aggregate |= item_has_aggregate;
                    bound_list.push(bound_item);
                }
                Ok((
                    Expression::InList {
                        expr: Box::new(bound_expr),
                        list: bound_list,
                        negated: *negated,
                    },
                    has_aggregate,
                ))
            },
            Expr::InSubquery {
                expr,
                subquery,
                negated,
            } => {
                let (bound_expr, has_aggregate) = self.build_expression(expr)?;
                let bound_subquery =
                    self.build_single_column_subquery_relation(subquery, "IN subquery")?;
                Ok((
                    Expression::InSubquery {
                        expr: Box::new(bound_expr),
                        subquery: Box::new(bound_subquery),
                        negated: *negated,
                    },
                    has_aggregate,
                ))
            },
            Expr::Exists { subquery, negated } => {
                let bound_subquery = self.build_subquery_relation(subquery)?;
                Ok((
                    Expression::Exists {
                        subquery: Box::new(bound_subquery),
                        negated: *negated,
                    },
                    false,
                ))
            },
            Expr::Subquery(query) => {
                let bound_subquery =
                    self.build_single_column_subquery_relation(query, "scalar subquery")?;
                Ok((Expression::ScalarSubquery(Box::new(bound_subquery)), false))
            },
            _ => Err(AnalyzerError::analysis(
                error_code::SCALAR_EXPRESSION_UNSUPPORTED,
                format!("unsupported scalar expression in this iteration: {expr}"),
            )),
        }
    }
}

fn map_binary_operator(operator: &BinaryOperator) -> Result<BoundBinaryOp, AnalyzerError> {
    let mapped = match operator {
        BinaryOperator::Plus => BoundBinaryOp::Add,
        BinaryOperator::Minus => BoundBinaryOp::Sub,
        BinaryOperator::Multiply => BoundBinaryOp::Mul,
        BinaryOperator::Divide => BoundBinaryOp::Div,
        BinaryOperator::Eq => BoundBinaryOp::Eq,
        BinaryOperator::NotEq => BoundBinaryOp::NotEq,
        BinaryOperator::Lt => BoundBinaryOp::Lt,
        BinaryOperator::LtEq => BoundBinaryOp::Lte,
        BinaryOperator::Gt => BoundBinaryOp::Gt,
        BinaryOperator::GtEq => BoundBinaryOp::Gte,
        BinaryOperator::And => BoundBinaryOp::And,
        BinaryOperator::Or => BoundBinaryOp::Or,
        _ => {
            return Err(AnalyzerError::analysis(
                error_code::BINARY_OPERATOR_UNSUPPORTED,
                format!("unsupported binary operator in this iteration: {operator}"),
            ));
        },
    };
    Ok(mapped)
}
