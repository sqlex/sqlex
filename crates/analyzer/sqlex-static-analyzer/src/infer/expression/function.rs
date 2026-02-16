use sqlex_analyzer::extension::DataTypeExt;
use sqlex_common::{dialect::Dialect, types::DataType};

use crate::{
    functions::model::{FunctionNullabilityRule, FunctionReturnTypeRule, FunctionSignature},
    infer::expression::ExpressionInference,
};

pub(super) fn infer_with_signature(
    signature: &FunctionSignature,
    args: &[ExpressionInference],
    dialect: Dialect,
) -> ExpressionInference {
    ExpressionInference {
        data_type: infer_return_type(signature.return_type_rule, args, dialect),
        nullable: infer_nullability(signature.nullability_rule, args),
    }
}

fn infer_return_type(
    return_type_rule: FunctionReturnTypeRule,
    args: &[ExpressionInference],
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

fn infer_nullability(
    nullability_rule: FunctionNullabilityRule,
    args: &[ExpressionInference],
) -> bool {
    match nullability_rule {
        FunctionNullabilityRule::AnyArg => args.iter().any(|arg| arg.nullable),
        FunctionNullabilityRule::AllArgs => args.iter().all(|arg| arg.nullable),
        FunctionNullabilityRule::Always => true,
        FunctionNullabilityRule::Never => false,
    }
}

pub(super) fn first_arg_type(args: &[ExpressionInference]) -> DataType {
    args.first()
        .map(|arg| arg.data_type.clone())
        .unwrap_or_else(unknown_type)
}

fn unknown_type() -> DataType {
    DataType::Custom("unknown".to_string())
}
