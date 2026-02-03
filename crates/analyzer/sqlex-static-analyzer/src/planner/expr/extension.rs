use sqlparser::ast::{Expr, ObjectName};

/// Extension trait for sqlparser Expr
pub trait ExprExt {
    fn has_aggregate_function(&self) -> bool;
}

impl ExprExt for Expr {
    fn has_aggregate_function(&self) -> bool {
        fn name_to_string(name: &ObjectName) -> String {
            name.0
                .iter()
                .map(|i| i.value.clone())
                .collect::<Vec<_>>()
                .join(".")
        }

        match self {
            Expr::Function(func) => {
                let name = name_to_string(&func.name).to_uppercase();
                matches!(
                    name.as_str(),
                    "COUNT"
                        | "SUM"
                        | "AVG"
                        | "MIN"
                        | "MAX"
                        | "ARRAY_AGG"
                        | "STRING_AGG"
                        | "JSON_AGG"
                )
            },
            Expr::BinaryOp { left, right, .. } => {
                left.has_aggregate_function() || right.has_aggregate_function()
            },
            Expr::UnaryOp { expr, .. } => expr.has_aggregate_function(),
            Expr::Nested(e) => e.has_aggregate_function(),
            Expr::Case {
                operand,
                conditions,
                results,
                else_result,
            } => {
                operand.as_ref().is_some_and(|e| e.has_aggregate_function())
                    || conditions.iter().any(|e| e.has_aggregate_function())
                    || results.iter().any(|e| e.has_aggregate_function())
                    || else_result
                        .as_ref()
                        .is_some_and(|e| e.has_aggregate_function())
            },
            _ => false,
        }
    }
}
