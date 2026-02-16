use sqlex_analyzer::extension::DataTypeExt;
use sqlex_common::types::DataType;

use crate::{
    algebraizer::model::expression::{BoundBinaryOp, BoundUnaryOp, Expression},
    diagnostics::{Diagnostic, Phase},
    infer::{
        Inferencer,
        expression::{
            function::{first_arg_type, infer_with_signature},
            literal::infer_literal_expression,
            slot::{infer_correlated_slot_expression, infer_slot_expression},
            type_rules::{boolean_result_type, validate_binary_op},
        },
        model::metadata::InferColumn,
    },
};

mod function;
mod literal;
mod slot;
mod subquery;
mod type_rules;

#[derive(Debug, Clone)]
pub(crate) struct ExpressionInference {
    pub(crate) data_type: DataType,
    pub(crate) nullable: bool,
}

impl Inferencer<'_> {
    pub(crate) fn infer_expression(
        &self,
        expr: &Expression,
        input_columns: &[InferColumn],
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<ExpressionInference, Diagnostic> {
        match expr {
            Expression::SlotRef(slot_id) => infer_slot_expression(*slot_id, input_columns),
            Expression::CorrelatedRef { depth, slot_id } => {
                infer_correlated_slot_expression(*depth, *slot_id, outer_scopes)
            },
            Expression::Literal(literal) => Ok(infer_literal_expression(literal, self.dialect)),
            Expression::BinaryOp { left, op, right } => {
                let left_info = self.infer_expression(left, input_columns, outer_scopes)?;
                let right_info = self.infer_expression(right, input_columns, outer_scopes)?;
                validate_binary_op(
                    op,
                    &left_info.data_type,
                    &right_info.data_type,
                    self.dialect,
                )?;

                let data_type = match op {
                    BoundBinaryOp::Add
                    | BoundBinaryOp::Sub
                    | BoundBinaryOp::Mul
                    | BoundBinaryOp::Div => {
                        left_info.data_type.promote_numeric(&right_info.data_type)
                    },
                    BoundBinaryOp::Eq
                    | BoundBinaryOp::NotEq
                    | BoundBinaryOp::Lt
                    | BoundBinaryOp::Lte
                    | BoundBinaryOp::Gt
                    | BoundBinaryOp::Gte
                    | BoundBinaryOp::And
                    | BoundBinaryOp::Or => boolean_result_type(self.dialect),
                };

                Ok(ExpressionInference {
                    data_type,
                    nullable: left_info.nullable || right_info.nullable,
                })
            },
            Expression::UnaryOp { op, expr } => {
                let info = self.infer_expression(expr, input_columns, outer_scopes)?;
                let data_type = match op {
                    BoundUnaryOp::Not => boolean_result_type(self.dialect),
                    BoundUnaryOp::Neg | BoundUnaryOp::Pos => info.data_type.clone(),
                };
                Ok(ExpressionInference {
                    data_type,
                    nullable: info.nullable,
                })
            },
            Expression::Function { name, args } => {
                let args_info = self.infer_expression_args(args, input_columns, outer_scopes)?;
                Ok(self.infer_function_expression(name, args_info))
            },
            Expression::AggregateCall {
                name,
                args,
                distinct,
            } => {
                let _ = distinct;
                let args_info = self.infer_expression_args(args, input_columns, outer_scopes)?;
                self.infer_aggregate_expression(name, args_info)
            },
            Expression::WindowCall { name, args, .. } => {
                let args_info = self.infer_expression_args(args, input_columns, outer_scopes)?;
                self.infer_window_call_expression(name, args_info)
            },
            Expression::Cast { expr, target_type } => {
                let info = self.infer_expression(expr, input_columns, outer_scopes)?;
                Ok(ExpressionInference {
                    data_type: target_type.clone(),
                    nullable: info.nullable,
                })
            },
            Expression::IsNull { .. } => Ok(ExpressionInference {
                data_type: boolean_result_type(self.dialect),
                nullable: false,
            }),
            Expression::Case {
                when_clauses,
                else_expr,
                ..
            } => {
                let mut branch_types = Vec::new();
                let mut nullable = false;

                for (condition, result) in when_clauses {
                    let _ = self.infer_expression(condition, input_columns, outer_scopes)?;
                    let result_info = self.infer_expression(result, input_columns, outer_scopes)?;
                    nullable |= result_info.nullable;
                    branch_types.push(result_info.data_type);
                }

                if let Some(else_expr) = else_expr {
                    let else_info =
                        self.infer_expression(else_expr, input_columns, outer_scopes)?;
                    nullable |= else_info.nullable;
                    branch_types.push(else_info.data_type);
                } else {
                    nullable = true;
                }

                let data_type = DataType::common_type(self.dialect, &branch_types)
                    .unwrap_or_else(|| DataType::Custom("unknown".to_string()));

                Ok(ExpressionInference {
                    data_type,
                    nullable,
                })
            },
            Expression::InList { expr, list, .. } => {
                let expr_info = self.infer_expression(expr, input_columns, outer_scopes)?;
                let mut nullable = expr_info.nullable;
                for item in list {
                    let item_info = self.infer_expression(item, input_columns, outer_scopes)?;
                    nullable |= item_info.nullable;
                }
                Ok(ExpressionInference {
                    data_type: boolean_result_type(self.dialect),
                    nullable,
                })
            },
            Expression::InSubquery {
                expr,
                subquery,
                negated,
            } => {
                let _ = negated;
                let expr_info = self.infer_expression(expr, input_columns, outer_scopes)?;
                let subquery_info = self.infer_single_column_subquery_expression(
                    subquery,
                    input_columns,
                    outer_scopes,
                )?;
                Ok(ExpressionInference {
                    data_type: boolean_result_type(self.dialect),
                    nullable: expr_info.nullable || subquery_info.nullable,
                })
            },
            Expression::Exists { subquery, negated } => {
                let _ = negated;
                let mut subquery_outer_scopes = outer_scopes.to_vec();
                subquery_outer_scopes.push(input_columns.to_vec());
                let _ = self.infer_relation_with_outer_scopes(subquery, &subquery_outer_scopes)?;
                Ok(ExpressionInference {
                    data_type: boolean_result_type(self.dialect),
                    nullable: false,
                })
            },
            Expression::ScalarSubquery(subquery) => {
                self.infer_single_column_subquery_expression(subquery, input_columns, outer_scopes)
            },
            Expression::Placeholder => Ok(ExpressionInference {
                data_type: DataType::Custom("unknown".to_string()),
                nullable: false,
            }),
        }
    }
}

impl Inferencer<'_> {
    fn infer_expression_args(
        &self,
        args: &[Expression],
        input_columns: &[InferColumn],
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<Vec<ExpressionInference>, Diagnostic> {
        let mut result = Vec::with_capacity(args.len());
        for arg in args {
            result.push(self.infer_expression(arg, input_columns, outer_scopes)?);
        }
        Ok(result)
    }

    fn infer_function_expression(
        &self,
        name: &str,
        args: Vec<ExpressionInference>,
    ) -> ExpressionInference {
        let Some(signature) = self.functions.resolve_scalar(name) else {
            let data_type = first_arg_type(&args);
            let nullable = args.iter().any(|arg| arg.nullable);
            return ExpressionInference {
                data_type,
                nullable,
            };
        };

        infer_with_signature(signature, &args, self.dialect)
    }

    fn infer_aggregate_expression(
        &self,
        name: &str,
        args: Vec<ExpressionInference>,
    ) -> Result<ExpressionInference, Diagnostic> {
        let Some(signature) = self.functions.resolve_aggregate(name) else {
            return Err(Diagnostic::new(
                "I4102",
                Phase::Infer,
                format!("unsupported aggregate function: {name}"),
            ));
        };

        Ok(infer_with_signature(signature, &args, self.dialect))
    }

    fn infer_window_call_expression(
        &self,
        name: &str,
        args: Vec<ExpressionInference>,
    ) -> Result<ExpressionInference, Diagnostic> {
        let Some(signature) = self.functions.resolve_window_call(name) else {
            return Err(Diagnostic::new(
                "I4103",
                Phase::Infer,
                format!("unsupported window function: {name}"),
            ));
        };

        Ok(infer_with_signature(signature, &args, self.dialect))
    }
}
