use sqlparser::ast::{Expr, FunctionArg, FunctionArgExpr, FunctionArguments, Value};

use crate::{
    analysis::{
        bind::{Binder, scope::BindScope},
        diagnostics::Diagnostic,
    },
    ir::bound::BoundExpr,
};

impl<'a> Binder<'a> {
    pub(super) fn bind_expr(&mut self, expr: &Expr, scope: &BindScope) -> crate::ir::ids::ExprId {
        match expr {
            Expr::Identifier(ident) => match scope.resolve_column(None, &ident.value) {
                Ok(col_id) => self.exprs.alloc(BoundExpr::Column(col_id)),
                Err(diag) => {
                    self.diagnostics.push(diag.with_context(expr.to_string()));
                    self.exprs.alloc(BoundExpr::Unsupported)
                },
            },
            Expr::CompoundIdentifier(idents) => {
                if idents.len() == 2 {
                    let table = &idents[0].value;
                    let col = &idents[1].value;
                    match scope.resolve_column(Some(table), col) {
                        Ok(col_id) => self.exprs.alloc(BoundExpr::Column(col_id)),
                        Err(diag) => {
                            self.diagnostics.push(diag.with_context(expr.to_string()));
                            self.exprs.alloc(BoundExpr::Unsupported)
                        },
                    }
                } else {
                    self.diagnostics.push(
                        Diagnostic::unsupported_feature("Deep compound identifiers")
                            .with_context(expr.to_string()),
                    );
                    self.exprs.alloc(BoundExpr::Unsupported)
                }
            },
            Expr::Value(value) => self.exprs.alloc(BoundExpr::Literal(value.clone())),
            Expr::BinaryOp { left, op, right } => {
                let left_id = self.bind_expr(left, scope);
                let right_id = self.bind_expr(right, scope);
                self.exprs.alloc(BoundExpr::Binary {
                    left: left_id,
                    op: op.clone(),
                    right: right_id,
                })
            },
            Expr::UnaryOp { op, expr: inner } => {
                let inner_id = self.bind_expr(inner, scope);
                self.exprs.alloc(BoundExpr::Unary {
                    op: *op,
                    expr: inner_id,
                })
            },
            Expr::Nested(inner) => self.bind_expr(inner, scope),
            Expr::IsNull(inner) => {
                let inner_id = self.bind_expr(inner, scope);
                self.exprs.alloc(BoundExpr::IsNull {
                    expr: inner_id,
                    negated: false,
                })
            },
            Expr::IsNotNull(inner) => {
                let inner_id = self.bind_expr(inner, scope);
                self.exprs.alloc(BoundExpr::IsNull {
                    expr: inner_id,
                    negated: true,
                })
            },
            Expr::Function(func) => {
                let name = func.name.to_string();
                let mut args = Vec::new();
                let mut distinct = false;
                let over = func.over.is_some();

                if let FunctionArguments::List(list) = &func.args {
                    distinct = list.duplicate_treatment.is_some();
                    for arg in &list.args {
                        match arg {
                            FunctionArg::Unnamed(FunctionArgExpr::Expr(expr)) => {
                                args.push(self.bind_expr(expr, scope));
                            },
                            FunctionArg::Named {
                                arg: FunctionArgExpr::Expr(expr),
                                ..
                            } => {
                                args.push(self.bind_expr(expr, scope));
                            },
                            FunctionArg::Unnamed(FunctionArgExpr::Wildcard) => {
                                let expr_id = self.exprs.alloc(BoundExpr::Literal(Value::Number(
                                    "1".to_string(),
                                    false,
                                )));
                                args.push(expr_id);
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

                self.exprs.alloc(BoundExpr::Function {
                    name,
                    args,
                    distinct,
                    over,
                })
            },
            Expr::Case {
                operand,
                conditions,
                results,
                else_result,
            } => {
                let operand_id = operand.as_ref().map(|expr| self.bind_expr(expr, scope));
                let cond_ids = conditions
                    .iter()
                    .map(|expr| self.bind_expr(expr, scope))
                    .collect::<Vec<_>>();
                let result_ids = results
                    .iter()
                    .map(|expr| self.bind_expr(expr, scope))
                    .collect::<Vec<_>>();
                let else_id = else_result.as_ref().map(|expr| self.bind_expr(expr, scope));

                if cond_ids.len() != result_ids.len() {
                    self.diagnostics.push(
                        Diagnostic::invalid_statement("CASE WHEN/THEN arity mismatch")
                            .with_context(expr.to_string()),
                    );
                }

                self.exprs.alloc(BoundExpr::Case {
                    operand: operand_id,
                    conditions: cond_ids,
                    results: result_ids,
                    else_result: else_id,
                })
            },
            Expr::Subquery(query) => {
                let bound = self.bind_subquery(query);
                self.exprs.alloc(BoundExpr::Subquery(Box::new(bound)))
            },
            _ => {
                self.diagnostics.push(
                    Diagnostic::unsupported_feature("expression in binder")
                        .with_context(expr.to_string()),
                );
                self.exprs.alloc(BoundExpr::Unsupported)
            },
        }
    }
}
