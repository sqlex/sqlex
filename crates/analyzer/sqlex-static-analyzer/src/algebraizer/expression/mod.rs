use sqlparser::ast::{BinaryOperator, Expr};

use crate::{
    algebraizer::{
        Algebraizer,
        context::BuildContext,
        model::expression::{BoundBinaryOp, BoundUnaryOp, Expression},
    },
    catalog::Catalog,
    diagnostics::{Diagnostic, Phase},
    functions::FunctionRegistry,
};

mod column;
mod function;
mod literal;
mod subquery;

impl Algebraizer {
    pub(crate) fn bind_expression(
        &self,
        expr: &Expr,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &BuildContext,
    ) -> Result<(Expression, bool), Diagnostic> {
        match expr {
            Expr::Identifier(ident) => {
                let normalized = crate::catalog::normalize::normalize_ident(ident, self.dialect);
                let binding = self.resolve_unqualified_column(&normalized, context)?;
                Ok((binding.into_scalar_expr(), false))
            },
            Expr::CompoundIdentifier(idents) => {
                if idents.is_empty() {
                    return Err(Diagnostic::new(
                        "A3004",
                        Phase::Algebraize,
                        "empty compound identifier",
                    ));
                }
                let column_name = crate::catalog::normalize::normalize_ident(
                    idents.last().expect("not empty"),
                    self.dialect,
                );
                let qualifier = idents[..idents.len() - 1]
                    .iter()
                    .map(|ident| crate::catalog::normalize::normalize_ident(ident, self.dialect))
                    .collect::<Vec<_>>()
                    .join(".");
                let binding = self.resolve_qualified_column(&qualifier, &column_name, context)?;
                Ok((binding.into_scalar_expr(), false))
            },
            Expr::Value(value) => Ok((
                Expression::Literal(self.bind_literal(value, context.literal_assignment_mode)?),
                false,
            )),
            Expr::Nested(inner) => self.bind_expression(inner, catalog, functions, context),
            Expr::UnaryOp { op, expr } => {
                let (inner, has_aggregate) =
                    self.bind_expression(expr, catalog, functions, context)?;
                let op = match op {
                    sqlparser::ast::UnaryOperator::Plus => BoundUnaryOp::Pos,
                    sqlparser::ast::UnaryOperator::Minus => BoundUnaryOp::Neg,
                    sqlparser::ast::UnaryOperator::Not => BoundUnaryOp::Not,
                    _ => {
                        return Err(Diagnostic::new(
                            "A3068",
                            Phase::Algebraize,
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
                let (left_expr, left_has_aggregate) =
                    self.bind_expression(left, catalog, functions, context)?;
                let (right_expr, right_has_aggregate) =
                    self.bind_expression(right, catalog, functions, context)?;
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
                let (inner, has_aggregate) =
                    self.bind_expression(expr, catalog, functions, context)?;
                let target_type =
                    crate::catalog::ddl_type_map::map_sql_data_type(self.dialect, data_type);
                Ok((
                    Expression::Cast {
                        expr: Box::new(inner),
                        target_type,
                    },
                    has_aggregate,
                ))
            },
            Expr::Ceil { expr, field } => {
                self.bind_ceil_or_floor("ceil", expr, field, catalog, functions, context)
            },
            Expr::Floor { expr, field } => {
                self.bind_ceil_or_floor("floor", expr, field, catalog, functions, context)
            },
            Expr::Trim {
                expr,
                trim_where,
                trim_what,
                trim_characters,
            } => {
                if trim_where.is_some() || trim_what.is_some() || trim_characters.is_some() {
                    return Err(Diagnostic::new(
                        "A3061",
                        Phase::Algebraize,
                        "TRIM modifiers are not supported in this iteration",
                    ));
                }
                let (bound_expr, has_aggregate) =
                    self.bind_expression(expr, catalog, functions, context)?;
                let bound_args = vec![bound_expr];
                let signature = functions.resolve_scalar("trim");
                self.validate_function_arity("trim", bound_args.len(), signature)?;
                self.validate_function_argument_types("trim", &bound_args, context, signature)?;
                Ok((
                    Expression::Function {
                        name: "trim".to_string(),
                        args: bound_args,
                    },
                    has_aggregate,
                ))
            },
            Expr::Function(function) => self.bind_function(function, catalog, functions, context),
            Expr::IsNull(inner) => {
                let (bound, has_aggregate) =
                    self.bind_expression(inner, catalog, functions, context)?;
                Ok((
                    Expression::IsNull {
                        expr: Box::new(bound),
                        negated: false,
                    },
                    has_aggregate,
                ))
            },
            Expr::IsNotNull(inner) => {
                let (bound, has_aggregate) =
                    self.bind_expression(inner, catalog, functions, context)?;
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
                        self.bind_expression(condition, catalog, functions, context)?;
                    let (bound_result, result_has_aggregate) =
                        self.bind_expression(result, catalog, functions, context)?;
                    has_aggregate |= condition_has_aggregate || result_has_aggregate;
                    when_clauses.push((bound_condition, bound_result));
                }

                let bound_operand = if let Some(operand) = operand {
                    let (bound, operand_has_aggregate) =
                        self.bind_expression(operand, catalog, functions, context)?;
                    has_aggregate |= operand_has_aggregate;
                    Some(Box::new(bound))
                } else {
                    None
                };

                let bound_else = if let Some(else_expr) = else_result {
                    let (bound, else_has_aggregate) =
                        self.bind_expression(else_expr, catalog, functions, context)?;
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
                let (bound_expr, mut has_aggregate) =
                    self.bind_expression(expr, catalog, functions, context)?;
                let mut bound_list = Vec::with_capacity(list.len());
                for item in list {
                    let (bound_item, item_has_aggregate) =
                        self.bind_expression(item, catalog, functions, context)?;
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
                let (bound_expr, has_aggregate) =
                    self.bind_expression(expr, catalog, functions, context)?;
                let bound_subquery = self.bind_single_column_subquery(
                    subquery,
                    catalog,
                    functions,
                    context,
                    "IN subquery",
                )?;
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
                let bound_subquery =
                    self.bind_subquery_relation(subquery, catalog, functions, context)?;
                Ok((
                    Expression::Exists {
                        subquery: Box::new(bound_subquery),
                        negated: *negated,
                    },
                    false,
                ))
            },
            Expr::Subquery(query) => {
                let bound_subquery = self.bind_single_column_subquery(
                    query,
                    catalog,
                    functions,
                    context,
                    "scalar subquery",
                )?;
                Ok((Expression::ScalarSubquery(Box::new(bound_subquery)), false))
            },
            _ => Err(Diagnostic::new(
                "A3070",
                Phase::Algebraize,
                format!("unsupported scalar expression in this iteration: {expr}"),
            )),
        }
    }
}

fn map_binary_operator(operator: &BinaryOperator) -> Result<BoundBinaryOp, Diagnostic> {
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
            return Err(Diagnostic::new(
                "A3069",
                Phase::Algebraize,
                format!("unsupported binary operator in this iteration: {operator}"),
            ));
        },
    };
    Ok(mapped)
}
