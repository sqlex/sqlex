use sqlparser::ast::{Expr, FunctionArg, FunctionArgExpr, FunctionArguments};

use super::Algebraizer;
use crate::{
    diagnostics::Diagnostic,
    functions::{self, Function},
    ir::{
        auxiliary::{BinaryOp, UnaryOp},
        scalar::{LiteralValue, ScalarExpr, WhenClause},
    },
};

impl<'a> Algebraizer<'a> {
    pub(super) fn build_scalar_expr(&mut self, expr: &Expr) -> ScalarExpr {
        match expr {
            Expr::Identifier(ident) => ScalarExpr::ColumnRef {
                table: None,
                column: ident.value.clone(),
            },
            Expr::CompoundIdentifier(idents) => {
                if idents.len() == 2 {
                    ScalarExpr::ColumnRef {
                        table: Some(idents[0].value.clone()),
                        column: idents[1].value.clone(),
                    }
                } else {
                    self.diagnostics.push(
                        Diagnostic::unsupported_feature("Deep compound identifiers")
                            .with_context(expr.to_string()),
                    );
                    ScalarExpr::Error
                }
            },
            Expr::Value(value) => self.build_literal(value),
            Expr::BinaryOp { left, op, right } => {
                let left_expr = self.build_scalar_expr(left);
                let right_expr = self.build_scalar_expr(right);
                ScalarExpr::BinaryOp {
                    left: Box::new(left_expr),
                    op: map_binary_op(op),
                    right: Box::new(right_expr),
                }
            },
            Expr::UnaryOp { op, expr: inner } => {
                let inner_expr = self.build_scalar_expr(inner);
                ScalarExpr::UnaryOp {
                    op: map_unary_op(op),
                    expr: Box::new(inner_expr),
                }
            },
            Expr::Nested(inner) => self.build_scalar_expr(inner),
            Expr::IsNull(inner) => {
                let inner_expr = self.build_scalar_expr(inner);
                ScalarExpr::IsNull {
                    expr: Box::new(inner_expr),
                    negated: false,
                }
            },
            Expr::IsNotNull(inner) => {
                let inner_expr = self.build_scalar_expr(inner);
                ScalarExpr::IsNull {
                    expr: Box::new(inner_expr),
                    negated: true,
                }
            },
            Expr::Function(func) => self.build_function_expr(func, expr),
            Expr::Case {
                operand,
                conditions,
                results,
                else_result,
            } => self.build_case_expr(operand, conditions, results, else_result, expr),
            Expr::Subquery(query) => {
                let rel = self.algebraize_query(query);
                match rel {
                    Some(r) => ScalarExpr::ScalarSubquery(Box::new(r)),
                    None => ScalarExpr::Error,
                }
            },
            Expr::InList {
                expr,
                list,
                negated,
            } => {
                let expr = self.build_scalar_expr(expr);
                let list = list.iter().map(|e| self.build_scalar_expr(e)).collect();
                ScalarExpr::InList {
                    expr: Box::new(expr),
                    list,
                    negated: *negated,
                }
            },
            Expr::InSubquery {
                expr,
                subquery,
                negated,
            } => {
                let scalar = self.build_scalar_expr(expr);
                let rel = self.algebraize_query(subquery);
                match rel {
                    Some(r) => ScalarExpr::InSubquery {
                        expr: Box::new(scalar),
                        subquery: Box::new(r),
                        negated: *negated,
                    },
                    None => ScalarExpr::Error,
                }
            },
            Expr::Trim {
                expr,
                trim_where,
                trim_what: _,
                trim_characters: _,
            } => {
                let inner = self.build_scalar_expr(expr);
                let func_name = match trim_where {
                    Some(sqlparser::ast::TrimWhereField::Leading) => "LTRIM",
                    Some(sqlparser::ast::TrimWhereField::Trailing) => "RTRIM",
                    Some(sqlparser::ast::TrimWhereField::Both) | None => "TRIM",
                };
                ScalarExpr::Function {
                    name: func_name.to_string(),
                    args: vec![inner],
                }
            },
            Expr::Ceil { expr, field: _ } => {
                let inner = self.build_scalar_expr(expr);
                ScalarExpr::Function {
                    name: "CEIL".to_string(),
                    args: vec![inner],
                }
            },
            Expr::Floor { expr, field: _ } => {
                let inner = self.build_scalar_expr(expr);
                ScalarExpr::Function {
                    name: "FLOOR".to_string(),
                    args: vec![inner],
                }
            },
            _ => {
                self.diagnostics.push(
                    Diagnostic::unsupported_feature("expression in algebraizer")
                        .with_context(expr.to_string()),
                );
                ScalarExpr::Error
            },
        }
    }

    fn build_literal(&mut self, value: &sqlparser::ast::Value) -> ScalarExpr {
        match value {
            sqlparser::ast::Value::Null => ScalarExpr::Literal(LiteralValue::Null),
            sqlparser::ast::Value::Boolean(b) => ScalarExpr::Literal(LiteralValue::Boolean(*b)),
            sqlparser::ast::Value::Number(num, _) => {
                if let Ok(i) = num.parse::<i64>() {
                    ScalarExpr::Literal(LiteralValue::Integer(i))
                } else if let Ok(f) = num.parse::<f64>() {
                    ScalarExpr::Literal(LiteralValue::Float(f))
                } else {
                    ScalarExpr::Literal(LiteralValue::String(num.clone()))
                }
            },
            sqlparser::ast::Value::SingleQuotedString(s)
            | sqlparser::ast::Value::DoubleQuotedString(s) => {
                ScalarExpr::Literal(LiteralValue::String(s.clone()))
            },
            _ => ScalarExpr::Literal(LiteralValue::Null),
        }
    }

    fn build_function_expr(&mut self, func: &sqlparser::ast::Function, expr: &Expr) -> ScalarExpr {
        let name = func.name.to_string();
        let upper = name.to_uppercase();
        let function = functions::resolve_function(&upper);

        let mut args = Vec::new();
        let mut distinct = false;
        let over = func.over.is_some();

        if let FunctionArguments::List(list) = &func.args {
            distinct = list.duplicate_treatment.is_some();
            for arg in &list.args {
                match arg {
                    FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) => {
                        args.push(self.build_scalar_expr(e));
                    },
                    FunctionArg::Named {
                        arg: FunctionArgExpr::Expr(e),
                        ..
                    } => {
                        args.push(self.build_scalar_expr(e));
                    },
                    FunctionArg::Unnamed(FunctionArgExpr::Wildcard) => {
                        args.push(ScalarExpr::Wildcard);
                    },
                    _ => {
                        self.diagnostics.push(
                            Diagnostic::unsupported_feature("function argument")
                                .with_context(expr.to_string()),
                        );
                    },
                }
            }
        }

        // Validate function at algebraize time
        if matches!(function, Function::Unknown) {
            self.diagnostics.push(Diagnostic::unknown_function(&upper));
        }
        if function.requires_over() && !over {
            self.diagnostics
                .push(Diagnostic::window_requires_over(&upper));
        }
        if over && !function.allows_over() {
            self.diagnostics.push(Diagnostic::over_not_allowed(&upper));
        }
        if distinct && !function.accepts_distinct() {
            self.diagnostics
                .push(Diagnostic::distinct_not_allowed(&upper));
        }
        if !function.arity().matches(args.len()) {
            self.diagnostics.push(Diagnostic::function_arity_mismatch(
                &upper,
                &function.arity().describe(),
                args.len(),
            ));
        }

        // Classify into the appropriate ScalarExpr variant
        let is_aggregate = matches!(function, Function::Aggregate(_));
        let is_window = matches!(function, Function::Window(_));

        if is_window || (over && is_aggregate) {
            // Window function call — extract OVER clause details
            let (partition_by, order_by) = self.extract_over_clause(&func.over);
            ScalarExpr::WindowCall {
                name,
                args,
                partition_by,
                order_by,
                is_aggregate_window: over && is_aggregate,
            }
        } else if is_aggregate && !over {
            ScalarExpr::AggregateCall {
                name,
                args,
                distinct,
            }
        } else {
            ScalarExpr::Function { name, args }
        }
    }

    fn extract_over_clause(
        &mut self,
        over: &Option<sqlparser::ast::WindowType>,
    ) -> (Vec<ScalarExpr>, Vec<crate::ir::auxiliary::SortKey>) {
        let Some(over) = over else {
            return (Vec::new(), Vec::new());
        };
        match over {
            sqlparser::ast::WindowType::WindowSpec(spec) => {
                let partition_by = spec
                    .partition_by
                    .iter()
                    .map(|e| self.build_scalar_expr(e))
                    .collect();
                let order_by = spec
                    .order_by
                    .iter()
                    .map(|ob| crate::ir::auxiliary::SortKey {
                        expr: self.build_scalar_expr(&ob.expr),
                        asc: ob.asc.unwrap_or(true),
                        nulls_first: ob.nulls_first,
                    })
                    .collect();
                (partition_by, order_by)
            },
            sqlparser::ast::WindowType::NamedWindow(_) => {
                self.diagnostics
                    .push(Diagnostic::unsupported_feature("named window reference"));
                (Vec::new(), Vec::new())
            },
        }
    }

    fn build_case_expr(
        &mut self,
        operand: &Option<Box<Expr>>,
        conditions: &[Expr],
        results: &[Expr],
        else_result: &Option<Box<Expr>>,
        expr: &Expr,
    ) -> ScalarExpr {
        let operand_expr = operand
            .as_ref()
            .map(|e| Box::new(self.build_scalar_expr(e)));

        if conditions.len() != results.len() {
            self.diagnostics.push(
                Diagnostic::invalid_statement("CASE WHEN/THEN arity mismatch")
                    .with_context(expr.to_string()),
            );
        }

        let when_clauses = conditions
            .iter()
            .zip(results.iter())
            .map(|(cond, result)| WhenClause {
                condition: self.build_scalar_expr(cond),
                result: self.build_scalar_expr(result),
            })
            .collect();

        let else_expr = else_result
            .as_ref()
            .map(|e| Box::new(self.build_scalar_expr(e)));

        ScalarExpr::Case {
            operand: operand_expr,
            when_clauses,
            else_result: else_expr,
        }
    }
}

fn map_binary_op(op: &sqlparser::ast::BinaryOperator) -> BinaryOp {
    use sqlparser::ast::BinaryOperator as SqlOp;
    match op {
        SqlOp::Plus => BinaryOp::Add,
        SqlOp::Minus => BinaryOp::Sub,
        SqlOp::Multiply => BinaryOp::Mul,
        SqlOp::Divide => BinaryOp::Div,
        SqlOp::Modulo => BinaryOp::Mod,
        SqlOp::Eq => BinaryOp::Eq,
        SqlOp::NotEq => BinaryOp::NotEq,
        SqlOp::Lt => BinaryOp::Lt,
        SqlOp::LtEq => BinaryOp::LtEq,
        SqlOp::Gt => BinaryOp::Gt,
        SqlOp::GtEq => BinaryOp::GtEq,
        SqlOp::Spaceship => BinaryOp::Spaceship,
        SqlOp::And => BinaryOp::And,
        SqlOp::Or => BinaryOp::Or,
        SqlOp::Xor => BinaryOp::Xor,
        SqlOp::StringConcat => BinaryOp::StringConcat,
        SqlOp::BitwiseOr => BinaryOp::BitwiseOr,
        SqlOp::BitwiseAnd => BinaryOp::BitwiseAnd,
        SqlOp::BitwiseXor => BinaryOp::BitwiseXor,
        SqlOp::PGBitwiseShiftLeft => BinaryOp::BitwiseShiftLeft,
        SqlOp::PGBitwiseShiftRight => BinaryOp::BitwiseShiftRight,
        SqlOp::PGRegexMatch => BinaryOp::PGRegexMatch,
        SqlOp::PGRegexIMatch => BinaryOp::PGRegexIMatch,
        SqlOp::PGRegexNotMatch => BinaryOp::PGRegexNotMatch,
        SqlOp::PGRegexNotIMatch => BinaryOp::PGRegexNotIMatch,
        SqlOp::PGLikeMatch => BinaryOp::PGLikeMatch,
        SqlOp::PGILikeMatch => BinaryOp::PGILikeMatch,
        SqlOp::PGNotLikeMatch => BinaryOp::PGNotLikeMatch,
        SqlOp::PGNotILikeMatch => BinaryOp::PGNotILikeMatch,
        SqlOp::PGStartsWith => BinaryOp::PGStartsWith,
        SqlOp::PGOverlap => BinaryOp::PGOverlap,
        SqlOp::Overlaps => BinaryOp::Overlaps,
        SqlOp::AtAt => BinaryOp::AtAt,
        SqlOp::AtArrow => BinaryOp::AtArrow,
        SqlOp::ArrowAt => BinaryOp::ArrowAt,
        SqlOp::AtQuestion => BinaryOp::AtQuestion,
        SqlOp::Question => BinaryOp::Question,
        SqlOp::QuestionAnd => BinaryOp::QuestionAnd,
        SqlOp::QuestionPipe => BinaryOp::QuestionPipe,
        // Default fallback for any unhandled operators
        _ => BinaryOp::Eq,
    }
}

fn map_unary_op(op: &sqlparser::ast::UnaryOperator) -> UnaryOp {
    use sqlparser::ast::UnaryOperator as SqlOp;
    match op {
        SqlOp::Not => UnaryOp::Not,
        SqlOp::Minus => UnaryOp::Neg,
        SqlOp::Plus => UnaryOp::Plus,
        _ => UnaryOp::Not,
    }
}
