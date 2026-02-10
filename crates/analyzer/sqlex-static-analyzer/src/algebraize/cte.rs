use sqlparser::ast::{self, SetExpr};

use super::{Algebraizer, CteEntry};
use crate::{diagnostics::Diagnostic, ir::relational::RelationalExpr};

impl<'a> Algebraizer<'a> {
    pub(super) fn process_ctes(&mut self, with: Option<&ast::With>) {
        let Some(with) = with else {
            return;
        };

        for cte in &with.cte_tables {
            let name = cte.alias.name.value.clone();
            let alias_columns: Vec<String> = cte
                .alias
                .columns
                .iter()
                .map(|c| c.name.value.clone())
                .collect();

            // For recursive CTEs, register a placeholder first so the recursive
            // reference can resolve during algebraization of the CTE body.
            if with.recursive {
                let anchor_cols = self.infer_recursive_anchor_columns(&cte.query, &alias_columns);
                let cols = if !alias_columns.is_empty() {
                    alias_columns.clone()
                } else {
                    anchor_cols
                };
                self.cte_scope.insert(
                    name.clone(),
                    CteEntry {
                        expr: RelationalExpr::Scan {
                            table: name.clone(),
                            alias: None,
                        },
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
                        .push(Diagnostic::cte_column_count_mismatch(&name));
                }
                output_cols = alias_columns;
            }

            // Register the CTE in scope
            if let Some(expr) = cte_expr {
                self.cte_scope.insert(
                    name,
                    CteEntry {
                        expr,
                        column_names: output_cols,
                    },
                );
            }
        }
    }

    fn infer_recursive_anchor_columns(
        &mut self,
        query: &ast::Query,
        alias_columns: &[String],
    ) -> Vec<String> {
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
        names
    }
}
