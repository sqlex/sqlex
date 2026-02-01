//! Expression type inference.

use sqlex_parser::{Expr, sqlparser};
use sqlex_types::SqlType;

use crate::{
    error::AnalyzeError,
    scope::{ColumnResolutionError, Scope},
};

/// Trait for resolving expression types.
pub trait TypeResolver {
    fn infer(&self, scope: &Scope, expr: &Expr) -> Result<(SqlType, bool), AnalyzeError>;
}

/// Default type resolver implementation.
pub struct DefaultTypeResolver;

impl TypeResolver for DefaultTypeResolver {
    fn infer(&self, scope: &Scope, expr: &Expr) -> Result<(SqlType, bool), AnalyzeError> {
        match expr {
            // Column reference
            Expr::Identifier(ident) => match scope.resolve_column(None, &ident.value) {
                Ok(col) => Ok((col.data_type.clone(), col.nullable)),
                Err(e) => Err(map_resolution_error(e)),
            },
            Expr::CompoundIdentifier(idents) => {
                if idents.len() == 2 {
                    let table = &idents[0].value;
                    let column = &idents[1].value;
                    match scope.resolve_column(Some(table), column) {
                        Ok(col) => Ok((col.data_type.clone(), col.nullable)),
                        Err(e) => Err(map_resolution_error(e)),
                    }
                } else {
                    Err(AnalyzeError::Unsupported(
                        "Compound identifier with > 2 parts".to_string(),
                    ))
                }
            },

            // Literals - sqlparser v0.55 uses ValueWithSpan
            Expr::Value(value_with_span) => Ok(Self::infer_value_with_span(value_with_span)),

            // Binary operations
            Expr::BinaryOp { left, op, right } => self.infer_binary_op(scope, left, op, right),

            // Unary operations
            Expr::UnaryOp { op, expr } => {
                let (inner_type, nullable) = self.infer(scope, expr)?;
                match op {
                    sqlparser::ast::UnaryOperator::Not => Ok((SqlType::Boolean, nullable)),
                    sqlparser::ast::UnaryOperator::Minus | sqlparser::ast::UnaryOperator::Plus => {
                        Ok((inner_type, nullable))
                    },
                    _ => Ok((inner_type, nullable)),
                }
            },

            // Function calls
            Expr::Function(func) => self.infer_function(scope, func),

            // CASE expression
            Expr::Case {
                conditions,
                else_result,
                ..
            } => {
                // Type is the common type of all results
                let mut result_type = SqlType::Unknown;
                let mut nullable = else_result.is_none(); // Nullable if no ELSE

                for case_when in conditions {
                    let (t, n) = self.infer(scope, &case_when.result)?;
                    if result_type == SqlType::Unknown {
                        result_type = t;
                    }
                    nullable = nullable || n;
                }

                if let Some(else_expr) = else_result {
                    let (t, n) = self.infer(scope, else_expr)?;
                    if result_type == SqlType::Unknown {
                        result_type = t;
                    }
                    nullable = nullable || n;
                }

                Ok((result_type, nullable))
            },

            // CAST
            Expr::Cast { data_type, .. } => {
                let sql_type =
                    sqlex_parser::convert_data_type(data_type, sqlex_types::Dialect::PostgreSQL);
                Ok((sql_type, false))
            },

            // Subquery
            Expr::Subquery(_) => {
                // Would need recursive analysis
                Ok((SqlType::Unknown, true))
            },

            // Nested expression
            Expr::Nested(inner) => self.infer(scope, inner),

            // IS NULL / IS NOT NULL
            Expr::IsNull(_) | Expr::IsNotNull(_) => Ok((SqlType::Boolean, false)),

            // IN list
            Expr::InList { .. } | Expr::InSubquery { .. } => Ok((SqlType::Boolean, false)),

            // BETWEEN
            Expr::Between { .. } => Ok((SqlType::Boolean, false)),

            // LIKE
            Expr::Like { .. } | Expr::ILike { .. } => Ok((SqlType::Boolean, false)),

            // EXISTS
            Expr::Exists { .. } => Ok((SqlType::Boolean, false)),

            // Default
            _ => Ok((SqlType::Unknown, true)),
        }
    }
}

impl DefaultTypeResolver {
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
        &self,
        scope: &Scope,
        left: &Expr,
        op: &sqlparser::ast::BinaryOperator,
        right: &Expr,
    ) -> Result<(SqlType, bool), AnalyzeError> {
        let (left_type, left_nullable) = self.infer(scope, left)?;
        let (right_type, right_nullable) = self.infer(scope, right)?;
        let nullable = left_nullable || right_nullable;

        use sqlparser::ast::BinaryOperator::*;
        match op {
            // Comparison operators return boolean
            Eq | NotEq | Lt | LtEq | Gt | GtEq => Ok((SqlType::Boolean, nullable)),

            // Logical operators return boolean
            And | Or | Xor => Ok((SqlType::Boolean, nullable)),

            // Arithmetic operators
            Plus | Minus | Multiply | Divide | Modulo => {
                if left_type.is_numeric() && right_type.is_numeric() {
                    Ok((SqlType::wider_numeric(&left_type, &right_type), nullable))
                } else {
                    Ok((SqlType::Unknown, nullable))
                }
            },

            // String concatenation
            StringConcat => Ok((SqlType::Text, nullable)),

            _ => Ok((SqlType::Unknown, nullable)),
        }
    }

    fn infer_function(
        &self,
        scope: &Scope,
        func: &sqlparser::ast::Function,
    ) -> Result<(SqlType, bool), AnalyzeError> {
        // Validate OVER clause if present
        if let Some(over) = &func.over {
            match over {
                sqlparser::ast::WindowType::WindowSpec(spec) => {
                    for expr in &spec.partition_by {
                        self.infer(scope, expr)?;
                    }
                    for order in &spec.order_by {
                        self.infer(scope, &order.expr)?;
                    }
                },
                sqlparser::ast::WindowType::NamedWindow(_) => {},
            }
        }

        let name = func
            .name
            .0
            .last()
            .map(|i| ident_to_string(i).to_uppercase())
            .unwrap_or_default();

        match name.as_str() {
            // COUNT always returns non-null BigInt
            "COUNT" => Ok((SqlType::BigInt, false)),

            // SUM preserves the input type
            "SUM" => {
                let inner_type = self.infer_function_arg_type(scope, func)?;
                Ok((inner_type, true)) // SUM returns NULL for empty set
            },

            // AVG returns Double (or Decimal for Decimal input)
            "AVG" => Ok((SqlType::Double, true)),

            // MIN/MAX preserve the input type
            "MIN" | "MAX" => {
                let inner_type = self.infer_function_arg_type(scope, func)?;
                Ok((inner_type, true))
            },

            // String functions
            "LOWER" | "UPPER" | "TRIM" | "LTRIM" | "RTRIM" | "CONCAT" | "SUBSTRING" | "SUBSTR" => {
                Ok((SqlType::Text, true))
            },

            // COALESCE - returns first non-null, type of first arg
            "COALESCE" => {
                let inner_type = self.infer_function_arg_type(scope, func)?;
                Ok((inner_type, false)) // COALESCE with literals makes it non-null
            },

            // NULLIF - makes result nullable
            "NULLIF" => {
                let inner_type = self.infer_function_arg_type(scope, func)?;
                Ok((inner_type, true))
            },

            // NOW, CURRENT_TIMESTAMP
            "NOW" | "CURRENT_TIMESTAMP" => Ok((SqlType::TimestampTz, false)),
            "CURRENT_DATE" => Ok((SqlType::Date, false)),
            "CURRENT_TIME" => Ok((SqlType::Time, false)),

            // Boolean functions
            "EXISTS" => Ok((SqlType::Boolean, false)),

            // Window functions - BigInt/Integer
            "ROW_NUMBER" | "RANK" | "DENSE_RANK" | "NTILE" => Ok((SqlType::BigInt, false)),

            // Window functions - preserve type
            "LAG" | "LEAD" | "FIRST_VALUE" | "LAST_VALUE" => {
                let inner_type = self.infer_function_arg_type(scope, func)?;
                Ok((inner_type, true))
            },

            // Set returning functions
            "UNNEST" => {
                let inner = self.infer_function_arg_type(scope, func)?;
                if let SqlType::Array(elem) = inner {
                    Ok((*elem, true))
                } else {
                    Ok((inner, true))
                }
            },

            // Default for unknown functions
            _ => Ok((SqlType::Unknown, true)),
        }
    }

    fn infer_function_arg_type(
        &self,
        scope: &Scope,
        func: &sqlparser::ast::Function,
    ) -> Result<SqlType, AnalyzeError> {
        // In sqlparser v0.55, args is FunctionArguments enum
        match &func.args {
            sqlparser::ast::FunctionArguments::List(list) => {
                if let Some(arg) = list.args.first() {
                    match arg {
                        sqlparser::ast::FunctionArg::Unnamed(
                            sqlparser::ast::FunctionArgExpr::Expr(expr),
                        ) => {
                            return Ok(self.infer(scope, expr)?.0);
                        },
                        sqlparser::ast::FunctionArg::Named {
                            arg: sqlparser::ast::FunctionArgExpr::Expr(expr),
                            ..
                        } => {
                            return Ok(self.infer(scope, expr)?.0);
                        },
                        _ => {},
                    }
                }
            },
            sqlparser::ast::FunctionArguments::Subquery(_) => {},
            sqlparser::ast::FunctionArguments::None => {},
        }
        Ok(SqlType::Unknown)
    }
}

fn map_resolution_error(err: ColumnResolutionError) -> AnalyzeError {
    match err {
        ColumnResolutionError::UnknownTable(t) => AnalyzeError::UnknownTable(t),
        ColumnResolutionError::UnknownColumn(c) => AnalyzeError::UnknownColumn(c),
        ColumnResolutionError::Ambiguous(c) => AnalyzeError::AmbiguousColumn(c),
    }
}

fn ident_to_string(ident: &sqlparser::ast::ObjectNamePart) -> String {
    match ident {
        sqlparser::ast::ObjectNamePart::Identifier(id) => id.value.clone(),
    }
}
