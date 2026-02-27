use sqlex_analyzer::{error::AnalyzerError, extension::data_type_ext::DataTypeExt};
use sqlex_common::types::DataType;

use crate::{
    algebraizer::model::expression::{BoundBinaryOp, BoundUnaryOp, Expression},
    infer::{
        Inferencer, error_code,
        expression::{
            function::{first_arg_type, infer_with_signature, validate_argument_types},
            literal::infer_literal_expression,
            slot::infer_slot_expression,
            type_rules::{boolean_result_type, validate_binary_op},
        },
        model::metadata::InferColumn,
    },
};

mod arithmetic;
mod function;
mod literal;
mod slot;
mod subquery;
mod type_rules;

#[derive(Debug, Clone)]
pub(crate) struct ExpressionInference {
    pub(crate) data_type: DataType,
    pub(crate) nullable: bool,
    pub(crate) int_literal_info: Option<IntLiteralInfo>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct IntLiteralInfo {
    pub(crate) value: i128,
    pub(crate) unsigned: bool,
    pub(crate) display_width: usize,
}

impl Inferencer<'_> {
    pub(crate) fn infer_expression(
        &mut self,
        expr: &Expression,
        input_columns: &[InferColumn],
    ) -> Result<ExpressionInference, AnalyzerError> {
        match expr {
            Expression::SlotRef(slot_id) => infer_slot_expression(*slot_id, input_columns),
            Expression::CorrelatedRef { depth, slot_id } => {
                self.infer_correlated_slot_expression(*depth, *slot_id)
            },
            Expression::Literal(literal) => Ok(infer_literal_expression(literal, self.dialect)),
            Expression::BinaryOp { left, op, right } => {
                let left_info = self.infer_expression(left, input_columns)?;
                let right_info = self.infer_expression(right, input_columns)?;
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
                    | BoundBinaryOp::Div => left_info
                        .data_type
                        .promote_numeric(self.dialect, &right_info.data_type),
                    BoundBinaryOp::Eq
                    | BoundBinaryOp::NotEq
                    | BoundBinaryOp::Lt
                    | BoundBinaryOp::Lte
                    | BoundBinaryOp::Gt
                    | BoundBinaryOp::Gte
                    | BoundBinaryOp::And
                    | BoundBinaryOp::Or => boolean_result_type(self.dialect),
                };
                // Arithmetic on pure int literals: compute result value for MySQL narrowing.
                // If both operands have IntLiteralInfo, the result does too (computed value).
                let int_literal_info = match op {
                    BoundBinaryOp::Add
                    | BoundBinaryOp::Sub
                    | BoundBinaryOp::Mul
                    | BoundBinaryOp::Div => {
                        self.fold_int_literal_binary(op, &left_info, &right_info)
                    },
                    _ => None,
                };

                Ok(ExpressionInference {
                    data_type,
                    nullable: left_info.nullable || right_info.nullable,
                    int_literal_info,
                })
            },
            Expression::UnaryOp { op, expr } => {
                let info = self.infer_expression(expr, input_columns)?;
                let data_type = match op {
                    BoundUnaryOp::Not => boolean_result_type(self.dialect),
                    BoundUnaryOp::Neg | BoundUnaryOp::Pos => info.data_type.clone(),
                };
                let nullable = info.nullable;
                // Unary +/- on pure int literals: preserve IntLiteralInfo for MySQL narrowing.
                let mysql_hint = info.int_literal_info;
                let int_literal_info = match op {
                    BoundUnaryOp::Not => None,
                    BoundUnaryOp::Pos => mysql_hint,
                    BoundUnaryOp::Neg => self.fold_int_literal_negate(mysql_hint),
                };
                Ok(ExpressionInference {
                    data_type,
                    nullable,
                    int_literal_info,
                })
            },
            Expression::Function { name, args } => {
                let mut args_info = Vec::with_capacity(args.len());
                for arg in args {
                    args_info.push(self.infer_expression(arg, input_columns)?);
                }
                self.infer_function_expression(name, args_info)
            },
            Expression::AggregateCall {
                name,
                args,
                distinct,
            } => {
                let _ = distinct;
                let mut args_info = Vec::with_capacity(args.len());
                for arg in args {
                    args_info.push(self.infer_expression(arg, input_columns)?);
                }
                self.infer_aggregate_expression(name, args_info)
            },
            Expression::WindowCall { name, args, .. } => {
                let mut args_info = Vec::with_capacity(args.len());
                for arg in args {
                    args_info.push(self.infer_expression(arg, input_columns)?);
                }
                self.infer_window_call_expression(name, args_info)
            },
            Expression::Cast { expr, target_type } => {
                let info = self.infer_expression(expr, input_columns)?;
                Ok(ExpressionInference {
                    data_type: target_type.clone(),
                    nullable: info.nullable,
                    int_literal_info: None,
                })
            },
            Expression::IsNull { .. } => Ok(ExpressionInference {
                data_type: boolean_result_type(self.dialect),
                nullable: false,
                int_literal_info: None,
            }),
            Expression::Case {
                when_clauses,
                else_expr,
                ..
            } => {
                let mut branch_types = Vec::new();
                let mut nullable = false;

                for (condition, result) in when_clauses {
                    let _ = self.infer_expression(condition, input_columns)?;
                    let result_info = self.infer_expression(result, input_columns)?;
                    nullable |= result_info.nullable;
                    branch_types.push(result_info.data_type);
                }

                if let Some(else_expr) = else_expr {
                    let else_info = self.infer_expression(else_expr, input_columns)?;
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
                    int_literal_info: None,
                })
            },
            Expression::InList { expr, list, .. } => {
                let expr_info = self.infer_expression(expr, input_columns)?;
                let mut nullable = expr_info.nullable;
                for item in list {
                    let item_info = self.infer_expression(item, input_columns)?;
                    nullable |= item_info.nullable;
                }
                Ok(ExpressionInference {
                    data_type: boolean_result_type(self.dialect),
                    nullable,
                    int_literal_info: None,
                })
            },
            Expression::InSubquery {
                expr,
                subquery,
                negated,
            } => {
                let _ = negated;
                let expr_info = self.infer_expression(expr, input_columns)?;
                let subquery_info =
                    self.infer_single_column_subquery_expression(subquery, input_columns)?;
                Ok(ExpressionInference {
                    data_type: boolean_result_type(self.dialect),
                    nullable: expr_info.nullable || subquery_info.nullable,
                    int_literal_info: None,
                })
            },
            Expression::Exists { subquery, negated } => {
                let _ = negated;
                self.outer_scopes.push(input_columns);
                let subquery_result = self.infer_relation(subquery);
                self.outer_scopes.pop();
                let _ = subquery_result?;
                Ok(ExpressionInference {
                    data_type: boolean_result_type(self.dialect),
                    nullable: false,
                    int_literal_info: None,
                })
            },
            Expression::ScalarSubquery(subquery) => {
                self.infer_single_column_subquery_expression(subquery, input_columns)
            },
            Expression::Placeholder => Ok(ExpressionInference {
                data_type: DataType::Custom("unknown".to_string()),
                nullable: false,
                int_literal_info: None,
            }),
        }
    }
}

impl Inferencer<'_> {
    fn infer_function_expression(
        &self,
        name: &str,
        args: Vec<ExpressionInference>,
    ) -> Result<ExpressionInference, AnalyzerError> {
        let Some(signature) = self.functions.resolve_scalar(name) else {
            let data_type = first_arg_type(&args);
            let nullable = args.iter().any(|arg| arg.nullable);
            return Ok(ExpressionInference {
                data_type,
                nullable,
                int_literal_info: None,
            });
        };

        validate_argument_types(name, signature, &args)?;
        Ok(infer_with_signature(signature, &args, self.dialect))
    }

    fn infer_aggregate_expression(
        &self,
        name: &str,
        args: Vec<ExpressionInference>,
    ) -> Result<ExpressionInference, AnalyzerError> {
        let Some(signature) = self.functions.resolve_aggregate(name) else {
            return Err(AnalyzerError::analysis(
                error_code::AGGREGATE_FUNCTION_UNSUPPORTED,
                format!("unsupported aggregate function: {name}"),
            ));
        };

        validate_argument_types(name, signature, &args)?;
        Ok(infer_with_signature(signature, &args, self.dialect))
    }

    fn infer_window_call_expression(
        &self,
        name: &str,
        args: Vec<ExpressionInference>,
    ) -> Result<ExpressionInference, AnalyzerError> {
        let Some(signature) = self.functions.resolve_window_call(name) else {
            return Err(AnalyzerError::analysis(
                error_code::WINDOW_FUNCTION_UNSUPPORTED,
                format!("unsupported window function: {name}"),
            ));
        };

        validate_argument_types(name, signature, &args)?;
        Ok(infer_with_signature(signature, &args, self.dialect))
    }
}
