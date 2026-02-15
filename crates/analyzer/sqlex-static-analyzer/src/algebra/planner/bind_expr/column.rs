use sqlex_common::dialect::Dialect;
use sqlparser::ast::{Expr, Value};

use crate::{
    algebra::{
        planner::{Algebraizer, context::BuildContext},
        scalar::BoundColumn,
    },
    catalog::normalize::{normalize_ident, normalize_object_name},
    diagnostics::{Diagnostic, Phase},
};

impl Algebraizer {
    pub(crate) fn resolve_unqualified_column<'a>(
        &self,
        column_name: &str,
        context: &'a BuildContext,
    ) -> Result<&'a BoundColumn, Diagnostic> {
        let columns = context.current_columns();
        let mut matched = columns
            .into_iter()
            .filter(|column| column.name == column_name);
        let first = matched.next();
        let second = matched.next();

        match (first, second) {
            (Some(column), None) => context
                .relation_scopes
                .iter()
                .flat_map(|scope| scope.schema.columns.iter())
                .find(|candidate| candidate.slot_id == column.slot_id)
                .ok_or_else(|| {
                    Diagnostic::new(
                        "A3007",
                        Phase::Algebraize,
                        format!("failed to resolve column '{column_name}'"),
                    )
                }),
            (None, _) => Err(Diagnostic::new(
                "A3008",
                Phase::Algebraize,
                format!("column not found: {column_name}"),
            )),
            (Some(_), Some(_)) => Err(Diagnostic::new(
                "A3009",
                Phase::Algebraize,
                format!("ambiguous column reference: {column_name}"),
            )),
        }
    }

    pub(crate) fn resolve_qualified_column<'a>(
        &self,
        qualifier: &str,
        column_name: &str,
        context: &'a BuildContext,
    ) -> Result<&'a BoundColumn, Diagnostic> {
        let Some(scope) = context.relation_scopes.iter().find(|scope| {
            scope
                .visible_names
                .iter()
                .any(|visible_name| visible_name == qualifier)
        }) else {
            return Err(Diagnostic::new(
                "A3010",
                Phase::Algebraize,
                format!("unknown relation reference: {qualifier}"),
            ));
        };

        scope
            .schema
            .columns
            .iter()
            .find(|column| column.name == column_name)
            .ok_or_else(|| {
                Diagnostic::new(
                    "A3011",
                    Phase::Algebraize,
                    format!("column not found: {qualifier}.{column_name}"),
                )
            })
    }

    pub(crate) fn derive_output_name(&self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Identifier(ident) => Ok(normalize_ident(ident, self.dialect)),
            Expr::CompoundIdentifier(idents) => {
                let last = idents.last().ok_or_else(|| {
                    Diagnostic::new(
                        "A3012",
                        Phase::Algebraize,
                        "empty compound identifier in projection",
                    )
                })?;
                Ok(normalize_ident(last, self.dialect))
            },
            Expr::Function(function) => {
                let function_name = normalize_object_name(&function.name, self.dialect);
                match self.dialect {
                    Dialect::Postgres => Ok(function_name.to_ascii_lowercase()),
                    Dialect::MySQL | Dialect::SQLite => Ok(expr.to_string()),
                }
            },
            Expr::Case { .. } => match self.dialect {
                Dialect::Postgres => Ok("case".to_string()),
                Dialect::MySQL | Dialect::SQLite => Ok(expr.to_string()),
            },
            Expr::Value(Value::SingleQuotedString(value)) => match self.dialect {
                Dialect::Postgres => Ok("?column?".to_string()),
                Dialect::MySQL => Ok(value.clone()),
                Dialect::SQLite => Ok(format!("'{value}'")),
            },
            Expr::Value(Value::Boolean(value)) => match self.dialect {
                Dialect::Postgres => Ok("?column?".to_string()),
                Dialect::MySQL | Dialect::SQLite => {
                    if *value {
                        Ok("TRUE".to_string())
                    } else {
                        Ok("FALSE".to_string())
                    }
                },
            },
            Expr::Value(_) => match self.dialect {
                Dialect::Postgres => Ok("?column?".to_string()),
                Dialect::MySQL | Dialect::SQLite => Ok(expr.to_string()),
            },
            _ => match self.dialect {
                Dialect::Postgres => Ok("?column?".to_string()),
                Dialect::MySQL | Dialect::SQLite => Ok(expr.to_string()),
            },
        }
    }
}
