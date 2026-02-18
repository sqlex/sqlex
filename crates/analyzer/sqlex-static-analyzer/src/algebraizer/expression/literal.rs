use sqlparser::ast::Value;

use crate::{
    algebraizer::{Algebraizer, model::expression::BoundLiteral},
    diagnostics::{Diagnostic, Phase},
};

impl Algebraizer<'_> {
    pub(crate) fn build_literal_expression(
        &self,
        value: &Value,
    ) -> Result<BoundLiteral, Diagnostic> {
        let literal = match value {
            Value::Boolean(boolean) => BoundLiteral::Bool(*boolean),
            Value::Null => BoundLiteral::Null,
            Value::SingleQuotedString(value)
            | Value::TripleSingleQuotedString(value)
            | Value::EscapedStringLiteral(value)
            | Value::UnicodeStringLiteral(value)
            | Value::NationalStringLiteral(value)
            | Value::DoubleQuotedString(value)
            | Value::TripleDoubleQuotedString(value)
            | Value::SingleQuotedRawStringLiteral(value)
            | Value::DoubleQuotedRawStringLiteral(value)
            | Value::TripleSingleQuotedRawStringLiteral(value)
            | Value::TripleDoubleQuotedRawStringLiteral(value) => {
                BoundLiteral::String(value.clone())
            },
            Value::Number(number, _) => {
                if number.contains('.') || number.contains('e') || number.contains('E') {
                    let parsed = number.parse::<f64>().map_err(|err| {
                        Diagnostic::new(
                            "A3005",
                            Phase::Algebraize,
                            format!("invalid floating literal '{number}': {err}"),
                        )
                    })?;
                    BoundLiteral::Float(parsed)
                } else {
                    let parsed = number.parse::<i64>().map_err(|err| {
                        Diagnostic::new(
                            "A3006",
                            Phase::Algebraize,
                            format!("invalid integer literal '{number}': {err}"),
                        )
                    })?;
                    BoundLiteral::Int {
                        value: parsed,
                        raw: number.clone(),
                    }
                }
            },
            Value::Placeholder(_) => BoundLiteral::Placeholder,
            _ => {
                return Err(Diagnostic::new(
                    "A3073",
                    Phase::Algebraize,
                    format!("unsupported literal in this iteration: {value}"),
                ));
            },
        };
        Ok(literal)
    }
}
