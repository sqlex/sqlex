use std::collections::HashSet;

use sqlex_common::dialect::Dialect;
use sqlparser::ast::{self, SetExpr};

use super::{Algebraizer, CteEntry};
use crate::{diagnostics::Diagnostic, ir::relational::RelationalExpr};

impl<'a> Algebraizer<'a> {
    pub(super) fn process_ctes(&mut self, with: Option<&ast::With>) {
        let Some(with) = with else {
            return;
        };

        let predeclare_all = with.recursive || self.dialect == Dialect::SQLite;
        let mut duplicate_names = HashSet::new();

        if predeclare_all {
            let mut seen_names = HashSet::new();

            // Pass 1: register all CTE names first so forward references can resolve.
            for cte in &with.cte_tables {
                let display_name = cte.alias.name.value.clone();
                let name = self.normalize_cte_name(&cte.alias.name);
                if !seen_names.insert(name.clone()) {
                    self.diagnostics.push(Diagnostic::invalid_statement(format!(
                        "Duplicate CTE name {display_name}"
                    )));
                    duplicate_names.insert(name);
                    continue;
                }

                let alias_columns: Vec<String> = cte
                    .alias
                    .columns
                    .iter()
                    .map(|c| c.name.value.clone())
                    .collect();

                self.cte_scopes.insert(
                    name.clone(),
                    CteEntry {
                        expr: RelationalExpr::Scan {
                            table: name,
                            alias: None,
                        },
                        column_names: alias_columns,
                    },
                );
            }

            // Pass 2: replace placeholders with anchor-derived placeholders so
            // recursive/self references can resolve expected column names.
            for cte in &with.cte_tables {
                let name = self.normalize_cte_name(&cte.alias.name);
                if duplicate_names.contains(&name) {
                    continue;
                }

                let alias_columns: Vec<String> = cte
                    .alias
                    .columns
                    .iter()
                    .map(|c| c.name.value.clone())
                    .collect();

                let (anchor_expr, anchor_cols) =
                    self.infer_recursive_anchor(&cte.query, &alias_columns);
                let cols = if !alias_columns.is_empty() {
                    alias_columns
                } else {
                    anchor_cols
                };
                let expr = anchor_expr.unwrap_or(RelationalExpr::Scan {
                    table: name.clone(),
                    alias: None,
                });

                self.cte_scopes.insert(
                    name,
                    CteEntry {
                        expr,
                        column_names: cols,
                    },
                );
            }
        }

        for cte in &with.cte_tables {
            let display_name = cte.alias.name.value.clone();
            let name = self.normalize_cte_name(&cte.alias.name);
            if duplicate_names.contains(&name) {
                continue;
            }

            if !predeclare_all && self.cte_scopes.contains_in_current(&name) {
                self.diagnostics.push(Diagnostic::invalid_statement(format!(
                    "Duplicate CTE name {display_name}"
                )));
                continue;
            }
            let alias_columns: Vec<String> = cte
                .alias
                .columns
                .iter()
                .map(|c| c.name.value.clone())
                .collect();

            // For recursive CTEs on non-predeclaration dialects, register a
            // placeholder first so the recursive reference can resolve.
            if with.recursive && !predeclare_all {
                let (anchor_expr, anchor_cols) =
                    self.infer_recursive_anchor(&cte.query, &alias_columns);
                let cols = if !alias_columns.is_empty() {
                    alias_columns.clone()
                } else {
                    anchor_cols
                };
                let expr = anchor_expr.unwrap_or(RelationalExpr::Scan {
                    table: name.clone(),
                    alias: None,
                });
                self.cte_scopes.insert(
                    name.clone(),
                    CteEntry {
                        expr,
                        column_names: cols,
                    },
                );
            }

            // Algebraize the CTE body
            let cte_expr = self.algebraize_query(&cte.query);

            // Determine output column names
            let mut output_cols = match &cte_expr {
                Some(expr) => self.output_names_for_expr(expr),
                None => Vec::new(),
            };

            if !alias_columns.is_empty() {
                if alias_columns.len() != output_cols.len() {
                    self.diagnostics
                        .push(Diagnostic::cte_column_count_mismatch(&display_name));
                }
                output_cols = alias_columns;
            }

            // Register the CTE in scope
            if let Some(expr) = cte_expr {
                self.cte_scopes.insert(
                    name,
                    CteEntry {
                        expr,
                        column_names: output_cols,
                    },
                );
            }
        }
    }

    fn infer_recursive_anchor(
        &mut self,
        query: &ast::Query,
        alias_columns: &[String],
    ) -> (Option<RelationalExpr>, Vec<String>) {
        // For recursive CTEs, the anchor is the left side of a UNION
        let anchor_set = match &*query.body {
            SetExpr::SetOperation { left, .. } => left.as_ref(),
            _ => query.body.as_ref(),
        };

        let anchor_expr = self.algebraize_set_expr(anchor_set);
        let mut names = match &anchor_expr {
            Some(expr) => self.output_names_for_expr(expr),
            None => Vec::new(),
        };

        if !alias_columns.is_empty() {
            names = alias_columns.to_vec();
        }
        (anchor_expr, names)
    }

    fn normalize_cte_name(&self, ident: &ast::Ident) -> String {
        match self.dialect {
            Dialect::Postgres if ident.quote_style.is_none() => ident.value.to_lowercase(),
            _ => ident.value.clone(),
        }
    }
}
