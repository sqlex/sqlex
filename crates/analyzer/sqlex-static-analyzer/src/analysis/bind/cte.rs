use sqlparser::ast::{self, Query, SetExpr};

use crate::{
    analysis::{
        bind::{Binder, CteBinding, CteDefEntry, scope::BindScope},
        diagnostics::Diagnostic,
    },
    ir::bound::BoundQueryBody,
};

impl<'a> Binder<'a> {
    pub(super) fn bind_ctes(&mut self, with: Option<&ast::With>) -> Vec<CteDefEntry> {
        let mut entries = self.cte_defs.clone();
        let Some(with) = with else {
            return entries;
        };

        for cte in &with.cte_tables {
            let name = cte.alias.name.value.clone();
            let alias_columns = cte
                .alias
                .columns
                .iter()
                .map(|c| c.name.value.clone())
                .collect::<Vec<_>>();

            if with.recursive {
                let anchor_cols = self.bind_recursive_anchor_columns(&cte.query, &alias_columns);
                let cols = if !alias_columns.is_empty() {
                    alias_columns.clone()
                } else {
                    anchor_cols
                };
                self.cte_scope
                    .insert(name.clone(), CteBinding { columns: cols });
            }

            let bound_query = self.bind_query_body(&cte.query);
            let mut output_cols = self.output_names_for_query_body(&bound_query);
            if !alias_columns.is_empty() {
                if alias_columns.len() != output_cols.len() {
                    self.diagnostics
                        .push(Diagnostic::cte_column_count_mismatch(&name));
                }
                output_cols = alias_columns.clone();
            }

            self.cte_scope.insert(
                name.clone(),
                CteBinding {
                    columns: output_cols.clone(),
                },
            );

            let entry = CteDefEntry {
                name,
                columns: output_cols,
                query: bound_query,
                recursive: with.recursive,
            };
            self.cte_defs.push(entry.clone());
            entries.push(entry);
        }

        entries
    }

    pub(super) fn bind_correlated_subquery_body(
        &mut self,
        query: &Query,
        outer_scope: &BindScope,
    ) -> BoundQueryBody {
        let prev_outer = self.outer_scopes.clone();
        let mut new_outers = vec![outer_scope.clone()];
        new_outers.extend(prev_outer.iter().cloned());
        self.outer_scopes = new_outers;

        let body = self.bind_query_body(query);

        self.outer_scopes = prev_outer;
        body
    }

    fn bind_recursive_anchor_columns(
        &mut self,
        query: &Query,
        alias_columns: &[String],
    ) -> Vec<String> {
        let anchor_set = match &*query.body {
            SetExpr::SetOperation { left, .. } => left.as_ref(),
            _ => query.body.as_ref(),
        };

        let anchor_body = self.bind_set_expr(anchor_set);
        let anchor_query = BoundQueryBody {
            body: anchor_body,
            order_by: Vec::new(),
            limit: None,
            offset: None,
        };

        let mut names = self.output_names_for_query_body(&anchor_query);
        if !alias_columns.is_empty() {
            names = alias_columns.to_vec();
        }
        names
    }
}
