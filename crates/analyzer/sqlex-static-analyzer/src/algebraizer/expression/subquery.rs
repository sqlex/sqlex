use sqlex_analyzer::{
    error::AnalyzerError,
    extension::{ident_ext::IdentExt, object_name_ext::ObjectNameExt},
};
use sqlex_common::{dialect::Dialect, types::DataType};
use sqlparser::ast::{Expr, Select, TableFactor};

use crate::algebraizer::{
    Algebraizer, error_code,
    model::{expression::BoundLiteral, relation::Relation},
};

impl Algebraizer<'_> {
    pub(crate) fn build_subquery_relation(
        &mut self,
        query: &sqlparser::ast::Query,
    ) -> Result<Relation, AnalyzerError> {
        self.build_query_relation(query)
    }

    pub(crate) fn build_single_column_subquery_relation(
        &mut self,
        query: &sqlparser::ast::Query,
        usage: &str,
    ) -> Result<Relation, AnalyzerError> {
        let relation = self.build_subquery_relation(query)?;
        let schema = relation.output_schema();
        if schema.columns.len() != 1 {
            return Err(AnalyzerError::analysis(
                error_code::SUBQUERY_EXPECTS_SINGLE_COLUMN,
                format!(
                    "{usage} expects subquery to return exactly one column, got {}",
                    schema.columns.len()
                ),
            ));
        }
        Ok(relation)
    }

    pub(crate) fn resolve_scalar_subquery_expr_type(
        &self,
        select: &Select,
        expr: &Expr,
    ) -> Option<(DataType, bool)> {
        match expr {
            Expr::Identifier(identifier) => {
                self.resolve_subquery_column(select, None, &identifier.value)
            },
            Expr::CompoundIdentifier(idents) => {
                if idents.len() < 2 {
                    return None;
                }
                let qualifier = idents[..idents.len() - 1]
                    .iter()
                    .map(|ident| ident.to_normalized_string(self.dialect))
                    .collect::<Vec<_>>()
                    .join(".");
                let column_name = idents.last()?.to_normalized_string(self.dialect);
                self.resolve_subquery_column(select, Some(&qualifier), &column_name)
            },
            Expr::Value(value) => {
                let bound_literal = self.build_literal_expression(value).ok()?;
                match bound_literal {
                    BoundLiteral::Null => Some((DataType::Custom("null".to_string()), true)),
                    BoundLiteral::Bool(_) => Some((
                        match self.dialect {
                            Dialect::Postgres => DataType::Bool,
                            Dialect::MySQL | Dialect::SQLite => DataType::BigInt,
                        },
                        false,
                    )),
                    BoundLiteral::Int { .. } => Some((
                        match self.dialect {
                            Dialect::Postgres => DataType::Int,
                            Dialect::MySQL => DataType::BigInt,
                            Dialect::SQLite => DataType::BigInt,
                        },
                        false,
                    )),
                    BoundLiteral::Float(_) => Some((
                        match self.dialect {
                            Dialect::SQLite => DataType::Double,
                            Dialect::MySQL | Dialect::Postgres => DataType::Decimal,
                        },
                        false,
                    )),
                    BoundLiteral::String(_) => Some((
                        match self.dialect {
                            Dialect::MySQL => DataType::Varchar,
                            Dialect::Postgres | Dialect::SQLite => DataType::Text,
                        },
                        false,
                    )),
                    BoundLiteral::Placeholder => {
                        Some((DataType::Custom("unknown".to_string()), false))
                    },
                }
            },
            _ => None,
        }
    }

    fn resolve_subquery_column(
        &self,
        select: &Select,
        qualifier: Option<&str>,
        column_name: &str,
    ) -> Option<(DataType, bool)> {
        if select.from.len() != 1 {
            return None;
        }

        let from_item = &select.from[0];
        if !from_item.joins.is_empty() {
            return None;
        }

        let TableFactor::Table { name, alias, .. } = &from_item.relation else {
            return None;
        };

        let normalized_table_name = name.to_normalized_string(self.dialect);
        let table = self.catalog.get_table(&normalized_table_name).ok()?;

        if let Some(qualifier) = qualifier {
            let mut qualifier_names = vec![normalized_table_name.clone()];
            if let Some(last_segment) = normalized_table_name.split('.').next_back() {
                if !qualifier_names.iter().any(|name| name == last_segment) {
                    qualifier_names.push(last_segment.to_string());
                }
            }
            if let Some(alias) = alias {
                qualifier_names.push(alias.name.to_normalized_string(self.dialect));
            }

            if !qualifier_names.iter().any(|name| name == qualifier) {
                return None;
            }
        }

        let normalized_column_name = normalize_column_name(column_name, self.dialect);
        let column = table
            .columns
            .iter()
            .find(|column| column.name == normalized_column_name)?;
        Some((column.data_type.clone(), column.nullable))
    }
}

fn normalize_column_name(name: &str, dialect: Dialect) -> String {
    if matches!(dialect, Dialect::Postgres) {
        name.to_ascii_lowercase()
    } else {
        name.to_string()
    }
}
