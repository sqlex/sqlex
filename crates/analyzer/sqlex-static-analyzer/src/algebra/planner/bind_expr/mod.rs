use sqlparser::ast::{BinaryOperator, Expr};

use crate::{
    algebra::{
        planner::{Algebraizer, context::BuildContext},
        scalar::{BoundBinaryOp, BoundScalarExpr, BoundUnaryOp},
    },
    catalog::model::Catalog,
    diagnostics::{Diagnostic, Phase},
    functions::registry::FunctionRegistry,
};

mod column;
mod function;
mod literal;
mod subquery;

impl Algebraizer {
    pub(crate) fn bind_expr(
        &self,
        expr: &Expr,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &BuildContext,
    ) -> Result<(BoundScalarExpr, bool), Diagnostic> {
        match expr {
            Expr::Identifier(ident) => {
                let normalized = crate::catalog::normalize::normalize_ident(ident, self.dialect);
                let column = self.resolve_unqualified_column(&normalized, context)?;
                Ok((BoundScalarExpr::SlotRef(column.slot_id), false))
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
                let column = self.resolve_qualified_column(&qualifier, &column_name, context)?;
                Ok((BoundScalarExpr::SlotRef(column.slot_id), false))
            },
            Expr::Value(value) => Ok((
                BoundScalarExpr::Literal(
                    self.bind_literal(value, context.literal_assignment_mode)?,
                ),
                false,
            )),
            Expr::Nested(inner) => self.bind_expr(inner, catalog, functions, context),
            Expr::UnaryOp { op, expr } => {
                let (inner, has_aggregate) = self.bind_expr(expr, catalog, functions, context)?;
                let op = match op {
                    sqlparser::ast::UnaryOperator::Plus => BoundUnaryOp::Pos,
                    sqlparser::ast::UnaryOperator::Minus => BoundUnaryOp::Neg,
                    sqlparser::ast::UnaryOperator::Not => BoundUnaryOp::Not,
                    _ => {
                        return Err(Diagnostic::todo(
                            Phase::Algebraize,
                            "this unary operator binding",
                        ));
                    },
                };
                Ok((
                    BoundScalarExpr::UnaryOp {
                        op,
                        expr: Box::new(inner),
                    },
                    has_aggregate,
                ))
            },
            Expr::BinaryOp { left, op, right } => {
                let (left_expr, left_has_aggregate) =
                    self.bind_expr(left, catalog, functions, context)?;
                let (right_expr, right_has_aggregate) =
                    self.bind_expr(right, catalog, functions, context)?;
                let op = map_binary_operator(op)?;
                Ok((
                    BoundScalarExpr::BinaryOp {
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
                let (inner, has_aggregate) = self.bind_expr(expr, catalog, functions, context)?;
                let target_type =
                    crate::catalog::ddl_type_map::map_sql_data_type(self.dialect, data_type);
                Ok((
                    BoundScalarExpr::Cast {
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
                    return Err(Diagnostic::todo(
                        Phase::Algebraize,
                        "TRIM modifiers binding",
                    ));
                }
                let (bound_expr, has_aggregate) =
                    self.bind_expr(expr, catalog, functions, context)?;
                let bound_args = vec![bound_expr];
                self.validate_function_arity("trim", bound_args.len(), functions)?;
                self.validate_function_argument_types("trim", &bound_args, context)?;
                Ok((
                    BoundScalarExpr::Function {
                        name: "trim".to_string(),
                        args: bound_args,
                    },
                    has_aggregate,
                ))
            },
            Expr::Function(function) => self.bind_function(function, catalog, functions, context),
            Expr::IsNull(inner) => {
                let (bound, has_aggregate) = self.bind_expr(inner, catalog, functions, context)?;
                Ok((
                    BoundScalarExpr::IsNull {
                        expr: Box::new(bound),
                        negated: false,
                    },
                    has_aggregate,
                ))
            },
            Expr::IsNotNull(inner) => {
                let (bound, has_aggregate) = self.bind_expr(inner, catalog, functions, context)?;
                Ok((
                    BoundScalarExpr::IsNull {
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
                        self.bind_expr(condition, catalog, functions, context)?;
                    let (bound_result, result_has_aggregate) =
                        self.bind_expr(result, catalog, functions, context)?;
                    has_aggregate |= condition_has_aggregate || result_has_aggregate;
                    when_clauses.push((bound_condition, bound_result));
                }

                let bound_operand = if let Some(operand) = operand {
                    let (bound, operand_has_aggregate) =
                        self.bind_expr(operand, catalog, functions, context)?;
                    has_aggregate |= operand_has_aggregate;
                    Some(Box::new(bound))
                } else {
                    None
                };

                let bound_else = if let Some(else_expr) = else_result {
                    let (bound, else_has_aggregate) =
                        self.bind_expr(else_expr, catalog, functions, context)?;
                    has_aggregate |= else_has_aggregate;
                    Some(Box::new(bound))
                } else {
                    None
                };

                Ok((
                    BoundScalarExpr::Case {
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
                    self.bind_expr(expr, catalog, functions, context)?;
                let mut bound_list = Vec::with_capacity(list.len());
                for item in list {
                    let (bound_item, item_has_aggregate) =
                        self.bind_expr(item, catalog, functions, context)?;
                    has_aggregate |= item_has_aggregate;
                    bound_list.push(bound_item);
                }
                Ok((
                    BoundScalarExpr::InList {
                        expr: Box::new(bound_expr),
                        list: bound_list,
                        negated: *negated,
                    },
                    has_aggregate,
                ))
            },
            Expr::InSubquery { expr, negated, .. } => {
                let (bound_expr, has_aggregate) =
                    self.bind_expr(expr, catalog, functions, context)?;
                Ok((
                    BoundScalarExpr::InSubquery {
                        expr: Box::new(bound_expr),
                        negated: *negated,
                    },
                    has_aggregate,
                ))
            },
            Expr::Exists { negated, .. } => {
                Ok((BoundScalarExpr::Exists { negated: *negated }, false))
            },
            Expr::Subquery(query) => {
                let (data_type, nullable) = self.infer_scalar_subquery_result(query, catalog);
                Ok((
                    BoundScalarExpr::ScalarSubquery {
                        data_type,
                        nullable,
                    },
                    false,
                ))
            },
            _ => Err(Diagnostic::todo(
                Phase::Algebraize,
                "this scalar expression binding",
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
            return Err(Diagnostic::todo(
                Phase::Algebraize,
                "this binary operator binding",
            ));
        },
    };
    Ok(mapped)
}
