use sqlex_analyzer::extension::DataTypeExt;
use sqlex_common::{dialect::Dialect, types::DataType};

use crate::{
    algebra::scalar::{BoundBinaryOp, BoundLiteral, BoundScalarExpr, BoundUnaryOp},
    catalog::model::Catalog,
    diagnostics::{Diagnostic, Phase},
    functions::registry::{
        FunctionNullabilityRule, FunctionRegistry, FunctionReturnTypeRule, FunctionSignature,
    },
    infer::{
        cardinality::MinRows, metadata::InferColumn,
        operator_infer::infer_operator_with_outer_scopes,
    },
};

#[derive(Debug, Clone)]
pub(crate) struct ScalarInference {
    pub(crate) data_type: DataType,
    pub(crate) nullable: bool,
}

pub(crate) fn infer_scalar(
    expr: &BoundScalarExpr,
    input_columns: &[InferColumn],
    catalog: &Catalog,
    dialect: Dialect,
    functions: &FunctionRegistry,
    outer_scopes: &[Vec<InferColumn>],
) -> Result<ScalarInference, Diagnostic> {
    match expr {
        BoundScalarExpr::SlotRef(slot_id) => infer_slot(*slot_id, input_columns),
        BoundScalarExpr::CorrelatedRef { depth, slot_id } => {
            infer_correlated_slot(*depth, *slot_id, outer_scopes)
        },
        BoundScalarExpr::Literal(literal) => Ok(infer_literal(literal, dialect)),
        BoundScalarExpr::BinaryOp { left, op, right } => {
            let left_info = infer_scalar(
                left,
                input_columns,
                catalog,
                dialect,
                functions,
                outer_scopes,
            )?;
            let right_info = infer_scalar(
                right,
                input_columns,
                catalog,
                dialect,
                functions,
                outer_scopes,
            )?;
            validate_binary_op(op, &left_info.data_type, &right_info.data_type, dialect)?;

            let data_type = match op {
                BoundBinaryOp::Add
                | BoundBinaryOp::Sub
                | BoundBinaryOp::Mul
                | BoundBinaryOp::Div => left_info.data_type.promote_numeric(&right_info.data_type),
                BoundBinaryOp::Eq
                | BoundBinaryOp::NotEq
                | BoundBinaryOp::Lt
                | BoundBinaryOp::Lte
                | BoundBinaryOp::Gt
                | BoundBinaryOp::Gte
                | BoundBinaryOp::And
                | BoundBinaryOp::Or => boolean_result_type(dialect),
            };

            Ok(ScalarInference {
                data_type,
                nullable: left_info.nullable || right_info.nullable,
            })
        },
        BoundScalarExpr::UnaryOp { op, expr } => {
            let info = infer_scalar(
                expr,
                input_columns,
                catalog,
                dialect,
                functions,
                outer_scopes,
            )?;
            let data_type = match op {
                BoundUnaryOp::Not => boolean_result_type(dialect),
                BoundUnaryOp::Neg | BoundUnaryOp::Pos => info.data_type.clone(),
            };
            Ok(ScalarInference {
                data_type,
                nullable: info.nullable,
            })
        },
        BoundScalarExpr::Function { name, args } => {
            let args_info = infer_args(
                args,
                input_columns,
                catalog,
                dialect,
                functions,
                outer_scopes,
            )?;
            Ok(infer_function(name, args_info, dialect, functions))
        },
        BoundScalarExpr::AggregateCall { name, args, .. } => {
            let args_info = infer_args(
                args,
                input_columns,
                catalog,
                dialect,
                functions,
                outer_scopes,
            )?;
            infer_aggregate(name, args_info, dialect, functions)
        },
        BoundScalarExpr::WindowCall { name, args, .. } => {
            let args_info = infer_args(
                args,
                input_columns,
                catalog,
                dialect,
                functions,
                outer_scopes,
            )?;
            infer_window(name, args_info, dialect, functions)
        },
        BoundScalarExpr::Cast { expr, target_type } => {
            let info = infer_scalar(
                expr,
                input_columns,
                catalog,
                dialect,
                functions,
                outer_scopes,
            )?;
            Ok(ScalarInference {
                data_type: target_type.clone(),
                nullable: info.nullable,
            })
        },
        BoundScalarExpr::IsNull { .. } => Ok(ScalarInference {
            data_type: boolean_result_type(dialect),
            nullable: false,
        }),
        BoundScalarExpr::Case {
            when_clauses,
            else_expr,
            ..
        } => {
            let mut branch_types = Vec::new();
            let mut nullable = false;

            for (condition, result) in when_clauses {
                let _ = infer_scalar(
                    condition,
                    input_columns,
                    catalog,
                    dialect,
                    functions,
                    outer_scopes,
                )?;
                let result_info = infer_scalar(
                    result,
                    input_columns,
                    catalog,
                    dialect,
                    functions,
                    outer_scopes,
                )?;
                nullable |= result_info.nullable;
                branch_types.push(result_info.data_type);
            }

            if let Some(else_expr) = else_expr {
                let else_info = infer_scalar(
                    else_expr,
                    input_columns,
                    catalog,
                    dialect,
                    functions,
                    outer_scopes,
                )?;
                nullable |= else_info.nullable;
                branch_types.push(else_info.data_type);
            } else {
                nullable = true;
            }

            let data_type = DataType::common_type(dialect, &branch_types)
                .unwrap_or_else(|| DataType::Custom("unknown".to_string()));

            Ok(ScalarInference {
                data_type,
                nullable,
            })
        },
        BoundScalarExpr::InList { expr, list, .. } => {
            let expr_info = infer_scalar(
                expr,
                input_columns,
                catalog,
                dialect,
                functions,
                outer_scopes,
            )?;
            let mut nullable = expr_info.nullable;
            for item in list {
                let item_info = infer_scalar(
                    item,
                    input_columns,
                    catalog,
                    dialect,
                    functions,
                    outer_scopes,
                )?;
                nullable |= item_info.nullable;
            }
            Ok(ScalarInference {
                data_type: boolean_result_type(dialect),
                nullable,
            })
        },
        BoundScalarExpr::InSubquery { expr, subquery, .. } => {
            let expr_info = infer_scalar(
                expr,
                input_columns,
                catalog,
                dialect,
                functions,
                outer_scopes,
            )?;
            let subquery_info = infer_subquery_single_column(
                subquery,
                input_columns,
                outer_scopes,
                catalog,
                dialect,
                functions,
            )?;
            Ok(ScalarInference {
                data_type: boolean_result_type(dialect),
                nullable: expr_info.nullable || subquery_info.nullable,
            })
        },
        BoundScalarExpr::Exists { subquery, .. } => {
            let mut subquery_outer_scopes = outer_scopes.to_vec();
            subquery_outer_scopes.push(input_columns.to_vec());
            let _ = infer_operator_with_outer_scopes(
                subquery,
                catalog,
                dialect,
                functions,
                &subquery_outer_scopes,
            )?;
            Ok(ScalarInference {
                data_type: boolean_result_type(dialect),
                nullable: false,
            })
        },
        BoundScalarExpr::ScalarSubquery(subquery) => infer_subquery_single_column(
            subquery,
            input_columns,
            outer_scopes,
            catalog,
            dialect,
            functions,
        ),
        BoundScalarExpr::Placeholder(_) => Ok(ScalarInference {
            data_type: DataType::Custom("unknown".to_string()),
            nullable: false,
        }),
    }
}

fn infer_subquery_single_column(
    subquery: &crate::algebra::expr::RelExpr,
    input_columns: &[InferColumn],
    outer_scopes: &[Vec<InferColumn>],
    catalog: &Catalog,
    dialect: Dialect,
    functions: &FunctionRegistry,
) -> Result<ScalarInference, Diagnostic> {
    let mut subquery_outer_scopes = outer_scopes.to_vec();
    subquery_outer_scopes.push(input_columns.to_vec());
    let metadata = infer_operator_with_outer_scopes(
        subquery,
        catalog,
        dialect,
        functions,
        &subquery_outer_scopes,
    )?;
    if metadata.columns.len() != 1 {
        return Err(Diagnostic::new(
            "I4105",
            Phase::Infer,
            format!(
                "subquery expression expects exactly one column, got {}",
                metadata.columns.len()
            ),
        ));
    }

    let column = &metadata.columns[0];
    Ok(ScalarInference {
        data_type: column.data_type.clone(),
        nullable: column.nullable || !matches!(metadata.cardinality.min, MinRows::One),
    })
}

fn validate_binary_op(
    op: &BoundBinaryOp,
    left_type: &DataType,
    right_type: &DataType,
    dialect: Dialect,
) -> Result<(), Diagnostic> {
    let is_arithmetic = matches!(
        op,
        BoundBinaryOp::Add | BoundBinaryOp::Sub | BoundBinaryOp::Mul | BoundBinaryOp::Div
    );
    if !is_arithmetic {
        return Ok(());
    }

    if dialect != Dialect::Postgres {
        return Ok(());
    }

    if is_unknown_type(left_type) || is_unknown_type(right_type) {
        return Ok(());
    }

    if left_type.is_numeric() && right_type.is_numeric() {
        return Ok(());
    }

    Err(Diagnostic::new(
        "I4104",
        Phase::Infer,
        format!(
            "operator '{}' is not defined for {:?} and {:?}",
            binary_op_symbol(op),
            left_type,
            right_type
        ),
    ))
}

fn binary_op_symbol(op: &BoundBinaryOp) -> &'static str {
    match op {
        BoundBinaryOp::Eq => "=",
        BoundBinaryOp::NotEq => "<>",
        BoundBinaryOp::Lt => "<",
        BoundBinaryOp::Lte => "<=",
        BoundBinaryOp::Gt => ">",
        BoundBinaryOp::Gte => ">=",
        BoundBinaryOp::Add => "+",
        BoundBinaryOp::Sub => "-",
        BoundBinaryOp::Mul => "*",
        BoundBinaryOp::Div => "/",
        BoundBinaryOp::And => "AND",
        BoundBinaryOp::Or => "OR",
    }
}

fn is_unknown_type(data_type: &DataType) -> bool {
    matches!(data_type, DataType::Custom(value) if value == "unknown" || value == "null")
}

fn infer_slot(slot_id: u32, input_columns: &[InferColumn]) -> Result<ScalarInference, Diagnostic> {
    let Some(column) = input_columns
        .iter()
        .find(|column| column.slot_id.is_some_and(|value| value == slot_id))
    else {
        return Err(Diagnostic::new(
            "I4101",
            Phase::Infer,
            format!("unknown slot reference: {slot_id}"),
        ));
    };

    Ok(ScalarInference {
        data_type: column.data_type.clone(),
        nullable: column.nullable,
    })
}

fn infer_correlated_slot(
    depth: usize,
    slot_id: u32,
    outer_scopes: &[Vec<InferColumn>],
) -> Result<ScalarInference, Diagnostic> {
    if depth == 0 || depth > outer_scopes.len() {
        return Err(Diagnostic::new(
            "I4106",
            Phase::Infer,
            format!("invalid correlated reference depth {depth} for slot {slot_id}"),
        ));
    }

    let scope_index = outer_scopes.len() - depth;
    let Some(column) = outer_scopes[scope_index]
        .iter()
        .find(|column| column.slot_id.is_some_and(|value| value == slot_id))
    else {
        return Err(Diagnostic::new(
            "I4107",
            Phase::Infer,
            format!("unknown correlated slot reference: slot {slot_id}, depth {depth}"),
        ));
    };

    Ok(ScalarInference {
        data_type: column.data_type.clone(),
        nullable: column.nullable,
    })
}

fn boolean_result_type(dialect: Dialect) -> DataType {
    match dialect {
        Dialect::Postgres => DataType::Bool,
        Dialect::MySQL | Dialect::SQLite => DataType::BigInt,
    }
}

fn infer_literal(literal: &BoundLiteral, dialect: Dialect) -> ScalarInference {
    match literal {
        BoundLiteral::Null => ScalarInference {
            data_type: DataType::Custom("null".to_string()),
            nullable: true,
        },
        BoundLiteral::Bool(_) => ScalarInference {
            data_type: match dialect {
                Dialect::Postgres => DataType::Bool,
                Dialect::MySQL | Dialect::SQLite => DataType::BigInt,
            },
            nullable: false,
        },
        BoundLiteral::Int {
            raw, assignment, ..
        } => ScalarInference {
            data_type: match dialect {
                Dialect::Postgres => DataType::Int,
                Dialect::MySQL => {
                    if *assignment && mysql_integer_literal_should_be_int(raw) {
                        DataType::Int
                    } else {
                        DataType::BigInt
                    }
                },
                Dialect::SQLite => DataType::BigInt,
            },
            nullable: false,
        },
        BoundLiteral::Float(_) => {
            let data_type = match dialect {
                Dialect::SQLite => DataType::Double,
                Dialect::MySQL | Dialect::Postgres => DataType::Decimal,
            };
            ScalarInference {
                data_type,
                nullable: false,
            }
        },
        BoundLiteral::String(_) => {
            let data_type = match dialect {
                Dialect::MySQL => DataType::Varchar,
                Dialect::Postgres | Dialect::SQLite => DataType::Text,
            };
            ScalarInference {
                data_type,
                nullable: false,
            }
        },
        BoundLiteral::Placeholder(_) => ScalarInference {
            data_type: DataType::Custom("unknown".to_string()),
            nullable: false,
        },
    }
}

fn mysql_integer_literal_should_be_int(raw: &str) -> bool {
    let digit_count = raw.chars().filter(|c| c.is_ascii_digit()).count();
    digit_count <= 8
}

fn infer_args(
    args: &[BoundScalarExpr],
    input_columns: &[InferColumn],
    catalog: &Catalog,
    dialect: Dialect,
    functions: &FunctionRegistry,
    outer_scopes: &[Vec<InferColumn>],
) -> Result<Vec<ScalarInference>, Diagnostic> {
    let mut result = Vec::with_capacity(args.len());
    for arg in args {
        result.push(infer_scalar(
            arg,
            input_columns,
            catalog,
            dialect,
            functions,
            outer_scopes,
        )?);
    }
    Ok(result)
}

fn infer_function(
    name: &str,
    args: Vec<ScalarInference>,
    dialect: Dialect,
    functions: &FunctionRegistry,
) -> ScalarInference {
    let Some(signature) = functions.resolve_scalar(name) else {
        let data_type = first_arg_type(&args);
        let nullable = args.iter().any(|arg| arg.nullable);
        return ScalarInference {
            data_type,
            nullable,
        };
    };

    infer_with_signature(signature, &args, dialect)
}

fn infer_aggregate(
    name: &str,
    args: Vec<ScalarInference>,
    dialect: Dialect,
    functions: &FunctionRegistry,
) -> Result<ScalarInference, Diagnostic> {
    let Some(signature) = functions.resolve_aggregate(name) else {
        return Err(Diagnostic::new(
            "I4102",
            Phase::Infer,
            format!("unsupported aggregate function: {name}"),
        ));
    };

    Ok(infer_with_signature(signature, &args, dialect))
}

fn infer_window(
    name: &str,
    args: Vec<ScalarInference>,
    dialect: Dialect,
    functions: &FunctionRegistry,
) -> Result<ScalarInference, Diagnostic> {
    let Some(signature) = functions.resolve_window_call(name) else {
        return Err(Diagnostic::new(
            "I4103",
            Phase::Infer,
            format!("unsupported window function: {name}"),
        ));
    };

    Ok(infer_with_signature(signature, &args, dialect))
}

fn infer_with_signature(
    signature: &FunctionSignature,
    args: &[ScalarInference],
    dialect: Dialect,
) -> ScalarInference {
    ScalarInference {
        data_type: infer_return_type(signature.return_type_rule, args, dialect),
        nullable: infer_nullability(signature.nullability_rule, args),
    }
}

fn infer_return_type(
    return_type_rule: FunctionReturnTypeRule,
    args: &[ScalarInference],
    dialect: Dialect,
) -> DataType {
    match return_type_rule {
        FunctionReturnTypeRule::TextLikeOrDefaultText => {
            let default_text_type = match dialect {
                Dialect::MySQL => DataType::Varchar,
                Dialect::Postgres | Dialect::SQLite => DataType::Text,
            };
            args.first()
                .map(|arg| arg.data_type.clone())
                .filter(|value| value.is_text_like())
                .unwrap_or(default_text_type)
        },
        FunctionReturnTypeRule::LengthInteger => match dialect {
            Dialect::Postgres => DataType::Int,
            Dialect::MySQL | Dialect::SQLite => DataType::BigInt,
        },
        FunctionReturnTypeRule::NumericUnary => {
            let first_type = first_arg_type(args);
            match dialect {
                Dialect::MySQL | Dialect::SQLite => {
                    if first_type.is_numeric() {
                        first_type
                    } else {
                        DataType::Double
                    }
                },
                Dialect::Postgres => first_type,
            }
        },
        FunctionReturnTypeRule::NumericBinaryCommon => {
            let arg_types = args
                .iter()
                .map(|arg| arg.data_type.clone())
                .collect::<Vec<_>>();
            let all_numeric = arg_types.iter().all(DataType::is_numeric);
            match dialect {
                Dialect::MySQL | Dialect::SQLite if !all_numeric => DataType::Double,
                _ => DataType::common_type(dialect, &arg_types).unwrap_or_else(unknown_type),
            }
        },
        FunctionReturnTypeRule::CoalesceCommonType => {
            let arg_types = args
                .iter()
                .map(|arg| arg.data_type.clone())
                .collect::<Vec<_>>();
            DataType::common_type(dialect, &arg_types).unwrap_or_else(unknown_type)
        },
        FunctionReturnTypeRule::NullIfFirstArg => first_arg_type(args),
        FunctionReturnTypeRule::Count => DataType::BigInt,
        FunctionReturnTypeRule::Sum => {
            let arg_type = first_arg_type(args);
            match dialect {
                Dialect::Postgres => {
                    if arg_type.is_integer() {
                        DataType::BigInt
                    } else {
                        arg_type
                    }
                },
                Dialect::MySQL | Dialect::SQLite => {
                    if arg_type.is_numeric() {
                        arg_type
                    } else {
                        DataType::Double
                    }
                },
            }
        },
        FunctionReturnTypeRule::Avg => DataType::Decimal,
        FunctionReturnTypeRule::MinMax | FunctionReturnTypeRule::LeadLag => first_arg_type(args),
        FunctionReturnTypeRule::Ranking => match dialect {
            Dialect::MySQL => DataType::UnsignedBigInt,
            Dialect::Postgres | Dialect::SQLite => DataType::BigInt,
        },
    }
}

fn infer_nullability(nullability_rule: FunctionNullabilityRule, args: &[ScalarInference]) -> bool {
    match nullability_rule {
        FunctionNullabilityRule::AnyArg => args.iter().any(|arg| arg.nullable),
        FunctionNullabilityRule::AllArgs => args.iter().all(|arg| arg.nullable),
        FunctionNullabilityRule::Always => true,
        FunctionNullabilityRule::Never => false,
    }
}

fn first_arg_type(args: &[ScalarInference]) -> DataType {
    args.first()
        .map(|arg| arg.data_type.clone())
        .unwrap_or_else(unknown_type)
}

fn unknown_type() -> DataType {
    DataType::Custom("unknown".to_string())
}
