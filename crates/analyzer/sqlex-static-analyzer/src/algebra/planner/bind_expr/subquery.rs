use sqlex_common::{dialect::Dialect, types::DataType};
use sqlparser::ast::{Expr, Select, TableFactor};

use crate::{
    algebra::{
        expr::RelExpr,
        planner::{Algebraizer, context::BuildContext},
        scalar::BoundLiteral,
    },
    catalog::{
        model::Catalog,
        normalize::{normalize_ident, normalize_object_name},
    },
    diagnostics::{Diagnostic, Phase},
    functions::registry::FunctionRegistry,
};

impl Algebraizer {
    pub(crate) fn bind_subquery_relation(
        &self,
        query: &sqlparser::ast::Query,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &BuildContext,
    ) -> Result<RelExpr, Diagnostic> {
        let mut subquery_context = BuildContext {
            relation_scopes: context.relation_scopes.clone(),
            outer_relation_scopes: context.outer_relation_scopes.clone(),
            next_relation_id: context.next_relation_id,
            next_slot_id: context.next_slot_id,
            ctes: context.ctes.clone(),
            literal_assignment_mode: true,
        };

        if let Some(with_clause) = &query.with {
            self.register_ctes(with_clause, catalog, functions, &mut subquery_context)?;
        }

        let relation =
            self.build_set_expr(&query.body, catalog, functions, &mut subquery_context)?;
        let relation = self.apply_top_level_order_by(
            relation,
            query,
            catalog,
            functions,
            &mut subquery_context,
        )?;
        self.apply_top_level_limit_offset(relation, query)
    }

    pub(crate) fn bind_single_column_subquery(
        &self,
        query: &sqlparser::ast::Query,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &BuildContext,
        usage: &str,
    ) -> Result<RelExpr, Diagnostic> {
        let relation = self.bind_subquery_relation(query, catalog, functions, context)?;
        let schema = super::super::output_schema_of(&relation)?;
        if schema.columns.len() != 1 {
            return Err(Diagnostic::new(
                "A3035",
                Phase::Algebraize,
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
        catalog: &Catalog,
    ) -> Option<(DataType, bool)> {
        match expr {
            Expr::Identifier(identifier) => {
                self.resolve_subquery_column(select, None, &identifier.value, catalog)
            },
            Expr::CompoundIdentifier(idents) => {
                if idents.len() < 2 {
                    return None;
                }
                let qualifier = idents[..idents.len() - 1]
                    .iter()
                    .map(|ident| normalize_ident(ident, self.dialect))
                    .collect::<Vec<_>>()
                    .join(".");
                let column_name = normalize_ident(idents.last()?, self.dialect);
                self.resolve_subquery_column(select, Some(&qualifier), &column_name, catalog)
            },
            Expr::Value(value) => {
                let bound_literal = self.bind_literal(value, false).ok()?;
                match bound_literal {
                    BoundLiteral::Null => Some((DataType::Custom("null".to_string()), true)),
                    BoundLiteral::Bool(_) => Some((
                        match self.dialect {
                            Dialect::Postgres => DataType::Bool,
                            Dialect::MySQL | Dialect::SQLite => DataType::BigInt,
                        },
                        false,
                    )),
                    BoundLiteral::Int {
                        raw, assignment, ..
                    } => Some((
                        match self.dialect {
                            Dialect::Postgres => DataType::Int,
                            Dialect::MySQL => {
                                if assignment
                                    && crate::algebra::planner::bind_expr::literal::mysql_integer_literal_should_be_int(&raw)
                                {
                                    DataType::Int
                                } else {
                                    DataType::BigInt
                                }
                            },
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
                    BoundLiteral::Placeholder(_) => {
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
        catalog: &Catalog,
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

        let normalized_table_name = normalize_object_name(name, self.dialect);
        let table = catalog.table(&normalized_table_name)?;

        if let Some(qualifier) = qualifier {
            let mut visible_names = vec![normalized_table_name.clone()];
            if let Some(last_segment) = normalized_table_name.split('.').next_back() {
                if !visible_names.iter().any(|name| name == last_segment) {
                    visible_names.push(last_segment.to_string());
                }
            }
            if let Some(alias) = alias {
                visible_names.push(normalize_ident(&alias.name, self.dialect));
            }

            if !visible_names.iter().any(|name| name == qualifier) {
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
