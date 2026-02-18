use sqlex_common::{dialect::Dialect, types::DataType};

use crate::{
    algebraizer::model::expression::BoundLiteral,
    infer::expression::{ExpressionInference, IntLiteralInfo, type_rules::boolean_result_type},
};

pub(super) fn infer_literal_expression(
    literal: &BoundLiteral,
    dialect: Dialect,
) -> ExpressionInference {
    match literal {
        BoundLiteral::Null => ExpressionInference {
            data_type: DataType::Custom("null".to_string()),
            nullable: true,
            int_literal_info: None,
        },
        BoundLiteral::Bool(_) => ExpressionInference {
            data_type: boolean_result_type(dialect),
            nullable: false,
            int_literal_info: None,
        },
        BoundLiteral::Int { value, raw } => ExpressionInference {
            data_type: match dialect {
                Dialect::Postgres => DataType::Int,
                Dialect::MySQL => DataType::BigInt,
                Dialect::SQLite => DataType::BigInt,
            },
            nullable: false,
            int_literal_info: match dialect {
                Dialect::MySQL => Some(IntLiteralInfo {
                    value: i128::from(*value),
                    unsigned: false,
                    display_width: raw.chars().filter(|ch| ch.is_ascii_digit()).count(),
                }),
                Dialect::Postgres | Dialect::SQLite => None,
            },
        },
        BoundLiteral::Float(_) => {
            let data_type = match dialect {
                Dialect::SQLite => DataType::Double,
                Dialect::MySQL | Dialect::Postgres => DataType::Decimal,
            };
            ExpressionInference {
                data_type,
                nullable: false,
                int_literal_info: None,
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
                int_literal_info: None,
            }
        },
        BoundLiteral::Placeholder => ExpressionInference {
            data_type: DataType::Custom("unknown".to_string()),
            nullable: false,
            int_literal_info: None,
        },
    }
}
