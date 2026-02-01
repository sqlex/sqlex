//! Expression type inference.

use sqlex_parser::{Expr, sqlparser};
use sqlex_types::SqlType;

use crate::scope::Scope;

/// Infer the type and nullability of an expression.
pub struct TypeInference;

impl TypeInference {
    /// Infer the type and nullability of an expression.
    /// Returns (SqlType, nullable).
    pub fn infer(scope: &Scope, expr: &Expr) -> (SqlType, bool) {
        match expr {
            // Column reference
            Expr::Identifier(ident) => match scope.resolve_column(None, &ident.value) {
                Ok(col) => (col.data_type.clone(), col.nullable),
                Err(_) => (SqlType::Unknown, true),
            },
            Expr::CompoundIdentifier(idents) => {
                if idents.len() == 2 {
                    let table = &idents[0].value;
                    let column = &idents[1].value;
                    match scope.resolve_column(Some(table), column) {
                        Ok(col) => (col.data_type.clone(), col.nullable),
                        Err(_) => (SqlType::Unknown, true),
                    }
                } else {
                    (SqlType::Unknown, true)
                }
            },

            // Literals - sqlparser v0.55 uses ValueWithSpan
            Expr::Value(value_with_span) => Self::infer_value_with_span(value_with_span),

            // Binary operations
            Expr::BinaryOp { left, op, right } => Self::infer_binary_op(scope, left, op, right),

            // Unary operations
            Expr::UnaryOp { op, expr } => {
                let (inner_type, nullable) = Self::infer(scope, expr);
                match op {
                    sqlparser::ast::UnaryOperator::Not => (SqlType::Boolean, nullable),
                    sqlparser::ast::UnaryOperator::Minus | sqlparser::ast::UnaryOperator::Plus => {
                        (inner_type, nullable)
                    },
                    _ => (inner_type, nullable),
                }
            },

            // Function calls
            Expr::Function(func) => Self::infer_function(scope, func),

            // CASE expression - in v0.55 it uses 'conditions' which are CaseWhen structs
            Expr::Case {
                conditions,
                else_result,
                ..
            } => {
                // Type is the common type of all results
                let mut result_type = SqlType::Unknown;
                let mut nullable = else_result.is_none(); // Nullable if no ELSE

                for case_when in conditions {
                    let (t, n) = Self::infer(scope, &case_when.result);
                    if result_type == SqlType::Unknown {
                        result_type = t;
                    }
                    nullable = nullable || n;
                }

                if let Some(else_expr) = else_result {
                    let (t, n) = Self::infer(scope, else_expr);
                    if result_type == SqlType::Unknown {
                        result_type = t;
                    }
                    nullable = nullable || n;
                }

                (result_type, nullable)
            },

            // CAST
            Expr::Cast { data_type, .. } => {
                let sql_type =
                    sqlex_parser::convert_data_type(data_type, sqlex_types::Dialect::PostgreSQL);
                (sql_type, false)
            },

            // Subquery
            Expr::Subquery(_) => {
                // Would need recursive analysis
                (SqlType::Unknown, true)
            },

            // Nested expression
            Expr::Nested(inner) => Self::infer(scope, inner),

            // IS NULL / IS NOT NULL
            Expr::IsNull(_) | Expr::IsNotNull(_) => (SqlType::Boolean, false),

            // IN list
            Expr::InList { .. } | Expr::InSubquery { .. } => (SqlType::Boolean, false),

            // BETWEEN
            Expr::Between { .. } => (SqlType::Boolean, false),

            // LIKE
            Expr::Like { .. } | Expr::ILike { .. } => (SqlType::Boolean, false),

            // EXISTS
            Expr::Exists { .. } => (SqlType::Boolean, false),

            // Default
            _ => (SqlType::Unknown, true),
        }
    }

    fn infer_value_with_span(value: &sqlparser::ast::ValueWithSpan) -> (SqlType, bool) {
        match &value.value {
            sqlparser::ast::Value::Number(_, _) => (SqlType::Integer, false), // Simplified
            sqlparser::ast::Value::SingleQuotedString(_)
            | sqlparser::ast::Value::DoubleQuotedString(_) => (SqlType::Text, false),
            sqlparser::ast::Value::Boolean(_) => (SqlType::Boolean, false),
            sqlparser::ast::Value::Null => (SqlType::Unknown, true),
            _ => (SqlType::Unknown, false),
        }
    }

    fn infer_binary_op(
        scope: &Scope,
        left: &Expr,
        op: &sqlparser::ast::BinaryOperator,
        right: &Expr,
    ) -> (SqlType, bool) {
        let (left_type, left_nullable) = Self::infer(scope, left);
        let (right_type, right_nullable) = Self::infer(scope, right);
        let nullable = left_nullable || right_nullable;

        use sqlparser::ast::BinaryOperator::*;
        match op {
            // Comparison operators return boolean
            Eq | NotEq | Lt | LtEq | Gt | GtEq => (SqlType::Boolean, nullable),

            // Logical operators return boolean
            And | Or | Xor => (SqlType::Boolean, nullable),

            // Arithmetic operators
            Plus | Minus | Multiply | Divide | Modulo => {
                if left_type.is_numeric() && right_type.is_numeric() {
                    (SqlType::wider_numeric(&left_type, &right_type), nullable)
                } else {
                    (SqlType::Unknown, nullable)
                }
            },

            // String concatenation
            StringConcat => (SqlType::Text, nullable),

            _ => (SqlType::Unknown, nullable),
        }
    }

    fn infer_function(scope: &Scope, func: &sqlparser::ast::Function) -> (SqlType, bool) {
        let name = func
            .name
            .0
            .last()
            .map(|i| ident_to_string(i).to_uppercase())
            .unwrap_or_default();

        match name.as_str() {
            // COUNT always returns non-null BigInt
            "COUNT" => (SqlType::BigInt, false),

            // SUM preserves the input type
            "SUM" => {
                let inner_type = Self::infer_function_arg_type(scope, func);
                (inner_type, true) // SUM returns NULL for empty set
            },

            // AVG returns Double (or Decimal for Decimal input)
            "AVG" => (SqlType::Double, true),

            // MIN/MAX preserve the input type
            "MIN" | "MAX" => {
                let inner_type = Self::infer_function_arg_type(scope, func);
                (inner_type, true)
            },

            // String functions
            "LOWER" | "UPPER" | "TRIM" | "LTRIM" | "RTRIM" | "CONCAT" | "SUBSTRING" | "SUBSTR" => {
                (SqlType::Text, true)
            },

            // COALESCE - returns first non-null, type of first arg
            "COALESCE" => {
                let inner_type = Self::infer_function_arg_type(scope, func);
                (inner_type, false) // COALESCE with literals makes it non-null
            },

            // NULLIF - makes result nullable
            "NULLIF" => {
                let inner_type = Self::infer_function_arg_type(scope, func);
                (inner_type, true)
            },

            // NOW, CURRENT_TIMESTAMP
            "NOW" | "CURRENT_TIMESTAMP" => (SqlType::TimestampTz, false),
            "CURRENT_DATE" => (SqlType::Date, false),
            "CURRENT_TIME" => (SqlType::Time, false),

            // Boolean functions
            "EXISTS" => (SqlType::Boolean, false),

            // Default for unknown functions
            _ => (SqlType::Unknown, true),
        }
    }

    fn infer_function_arg_type(scope: &Scope, func: &sqlparser::ast::Function) -> SqlType {
        // In sqlparser v0.55, args is FunctionArguments enum
        match &func.args {
            sqlparser::ast::FunctionArguments::List(list) => {
                if let Some(arg) = list.args.first() {
                    match arg {
                        sqlparser::ast::FunctionArg::Unnamed(
                            sqlparser::ast::FunctionArgExpr::Expr(expr),
                        ) => {
                            return Self::infer(scope, expr).0;
                        },
                        sqlparser::ast::FunctionArg::Named {
                            arg: sqlparser::ast::FunctionArgExpr::Expr(expr),
                            ..
                        } => {
                            return Self::infer(scope, expr).0;
                        },
                        _ => {},
                    }
                }
            },
            sqlparser::ast::FunctionArguments::Subquery(_) => {},
            sqlparser::ast::FunctionArguments::None => {},
        }
        SqlType::Unknown
    }
}

fn ident_to_string(ident: &sqlparser::ast::ObjectNamePart) -> String {
    match ident {
        sqlparser::ast::ObjectNamePart::Identifier(id) => id.value.clone(),
    }
}
