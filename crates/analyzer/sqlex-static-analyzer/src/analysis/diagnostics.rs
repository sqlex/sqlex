#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    ParseError,
    UnsupportedFeature,
    InvalidStatement,
    UnknownTable,
    UnknownTableAlias,
    UnknownColumn,
    AmbiguousColumn,
    InvalidOrderByPosition,
    InvalidGrouping,
    InvalidFunctionUsage,
    UnknownFunction,
    InvalidSubquery,
    InvalidSetOperation,
    InvalidValues,
    InvalidJoin,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub code: Option<DiagnosticCode>,
    pub context: Option<String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            message: message.into(),
            code: None,
            context: None,
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            message: message.into(),
            code: None,
            context: None,
        }
    }

    pub fn error_with_code(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            message: message.into(),
            code: Some(code),
            context: None,
        }
    }

    pub fn warning_with_code(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            message: message.into(),
            code: Some(code),
            context: None,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        if let Some(code) = self.code {
            out.push('[');
            out.push_str(&format!("{code:?}"));
            out.push_str("] ");
        }
        out.push_str(&self.message);
        if let Some(ctx) = &self.context {
            out.push_str(" (");
            out.push_str(ctx);
            out.push(')');
        }
        out
    }

    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::error_with_code(DiagnosticCode::ParseError, message)
    }

    pub fn unsupported_feature(feature: impl Into<String>) -> Self {
        Self::warning_with_code(
            DiagnosticCode::UnsupportedFeature,
            format!("Unsupported feature: {}", feature.into()),
        )
    }

    pub fn invalid_statement(message: impl Into<String>) -> Self {
        Self::error_with_code(DiagnosticCode::InvalidStatement, message)
    }

    pub fn unknown_table(name: &str) -> Self {
        Self::error_with_code(
            DiagnosticCode::UnknownTable,
            format!("Table {name} not found in catalog"),
        )
    }

    pub fn unknown_cte(name: &str) -> Self {
        Self::error_with_code(
            DiagnosticCode::UnknownTable,
            format!("CTE {name} not found"),
        )
    }

    pub fn unknown_table_alias(alias: &str) -> Self {
        Self::error_with_code(
            DiagnosticCode::UnknownTableAlias,
            format!("Unknown table alias {alias}"),
        )
    }

    pub fn unknown_column(column: &str) -> Self {
        Self::error_with_code(
            DiagnosticCode::UnknownColumn,
            format!("Column {column} not found"),
        )
    }

    pub fn ambiguous_column(column: &str) -> Self {
        Self::error_with_code(
            DiagnosticCode::AmbiguousColumn,
            format!("Ambiguous column {column}"),
        )
    }

    pub fn derived_table_requires_alias() -> Self {
        Self::error_with_code(
            DiagnosticCode::InvalidStatement,
            "Derived table must have an alias",
        )
    }

    pub fn join_using_column_missing(column: &str) -> Self {
        Self::error_with_code(
            DiagnosticCode::InvalidJoin,
            format!("JOIN USING column {column} not found on both sides"),
        )
    }

    pub fn unknown_function(name: &str) -> Self {
        Self::error_with_code(
            DiagnosticCode::UnknownFunction,
            format!("Unknown function: {name}"),
        )
    }

    pub fn window_requires_over(name: &str) -> Self {
        Self::error_with_code(
            DiagnosticCode::InvalidFunctionUsage,
            format!("Window function {name} requires an OVER clause"),
        )
    }

    pub fn over_not_allowed(name: &str) -> Self {
        Self::error_with_code(
            DiagnosticCode::InvalidFunctionUsage,
            format!("OVER is not allowed for scalar function {name}"),
        )
    }

    pub fn distinct_not_allowed(name: &str) -> Self {
        Self::error_with_code(
            DiagnosticCode::InvalidFunctionUsage,
            format!("DISTINCT is only allowed for aggregate functions (found in {name})"),
        )
    }

    pub fn function_arity_mismatch(name: &str, expected: &str, found: usize) -> Self {
        Self::error_with_code(
            DiagnosticCode::InvalidFunctionUsage,
            format!("Function {name} expects {expected} arguments, found {found}"),
        )
    }

    pub fn function_argument_type_mismatch(name: &str, detail: impl Into<String>) -> Self {
        Self::error_with_code(
            DiagnosticCode::InvalidFunctionUsage,
            format!(
                "Function {name} has invalid argument types: {}",
                detail.into()
            ),
        )
    }

    pub fn binary_operator_type_mismatch(operator: &str, left: &str, right: &str) -> Self {
        Self::error_with_code(
            DiagnosticCode::InvalidStatement,
            format!("Operator {operator} is not supported for types {left} and {right}"),
        )
    }

    pub fn aggregate_not_allowed(context: &str) -> Self {
        Self::error_with_code(
            DiagnosticCode::InvalidGrouping,
            format!("Aggregate functions are not allowed in {context}"),
        )
        .with_context(context)
    }

    pub fn window_not_allowed(context: &str) -> Self {
        Self::error_with_code(
            DiagnosticCode::InvalidGrouping,
            format!("Window functions are not allowed in {context}"),
        )
        .with_context(context)
    }

    pub fn group_by_aggregate_not_allowed() -> Self {
        Self::error_with_code(
            DiagnosticCode::InvalidGrouping,
            "Aggregate functions are not allowed in GROUP BY",
        )
        .with_context("GROUP BY")
    }

    pub fn group_by_window_not_allowed() -> Self {
        Self::error_with_code(
            DiagnosticCode::InvalidGrouping,
            "Window functions are not allowed in GROUP BY",
        )
        .with_context("GROUP BY")
    }

    pub fn order_by_position_out_of_range(position: usize) -> Self {
        Self::error_with_code(
            DiagnosticCode::InvalidOrderByPosition,
            format!("ORDER BY position {position} is out of range"),
        )
    }

    pub fn grouping_error(message: impl Into<String>) -> Self {
        Self::error_with_code(DiagnosticCode::InvalidGrouping, message)
    }

    pub fn scalar_subquery_column_count() -> Self {
        Self::error_with_code(
            DiagnosticCode::InvalidSubquery,
            "Scalar subquery must return exactly one column",
        )
    }

    pub fn values_column_count_mismatch() -> Self {
        Self::error_with_code(
            DiagnosticCode::InvalidValues,
            "VALUES rows have mismatched column counts",
        )
    }

    pub fn set_operation_column_count_mismatch() -> Self {
        Self::error_with_code(
            DiagnosticCode::InvalidSetOperation,
            "Set operation column count mismatch",
        )
    }

    pub fn cte_column_count_mismatch(name: &str) -> Self {
        Self::error_with_code(
            DiagnosticCode::InvalidStatement,
            format!("CTE {name} column count mismatch"),
        )
    }
}
