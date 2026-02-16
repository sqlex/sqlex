use sqlex_common::{dialect::Dialect, types::DataType};

use crate::{
    algebraizer::model::expression::BoundLiteral,
    infer::expression::{ExpressionInference, type_rules::boolean_result_type},
};

pub(super) fn infer_literal_expression(
    literal: &BoundLiteral,
    dialect: Dialect,
) -> ExpressionInference {
    match literal {
        BoundLiteral::Null => ExpressionInference {
            data_type: DataType::Custom("null".to_string()),
            nullable: true,
        },
        BoundLiteral::Bool(_) => ExpressionInference {
            data_type: boolean_result_type(dialect),
            nullable: false,
        },
        BoundLiteral::Int {
            raw, assignment, ..
        } => ExpressionInference {
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
            ExpressionInference {
                data_type,
                nullable: false,
            }
        },
        BoundLiteral::String(_) => {
            let data_type = match dialect {
                Dialect::MySQL => DataType::Varchar,
                Dialect::Postgres | Dialect::SQLite => DataType::Text,
            };
            ExpressionInference {
                data_type,
                nullable: false,
            }
        },
        BoundLiteral::Placeholder => ExpressionInference {
            data_type: DataType::Custom("unknown".to_string()),
            nullable: false,
        },
    }
}

fn mysql_integer_literal_should_be_int(raw: &str) -> bool {
    let digit_count = raw.chars().filter(|c| c.is_ascii_digit()).count();
    digit_count <= 8
}
