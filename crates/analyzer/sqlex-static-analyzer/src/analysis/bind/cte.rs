use std::{mem, sync::Arc};

use sqlparser::ast::{self, Query, SetExpr};

use crate::{
    analysis::{
        bind::{Binder, CteBinding, scope::BindScope},
        diagnostics::Diagnostic,
    },
    ir::bound::{BoundCte, BoundQuery, BoundSetExpr},
};

impl<'a> Binder<'a> {
    pub(super) fn bind_ctes(&mut self, with: Option<&ast::With>) -> Vec<Arc<BoundCte>> {
        let mut ctes = self.cte_defs.clone();
        let Some(with) = with else {
            return ctes;
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
                let mut anchor_cols =
                    self.bind_recursive_anchor_columns(&cte.query, &alias_columns);
                if !alias_columns.is_empty() {
                    anchor_cols = alias_columns.clone();
                }
                self.cte_scope.insert(
                    name.clone(),
                    CteBinding {
                        columns: anchor_cols,
                    },
                );
            }

            let bound_query = self.bind_subquery(&cte.query);
            let mut output_cols = self.output_names_for_query(&bound_query);
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

            let cte_def = Arc::new(BoundCte {
                name,
                columns: output_cols,
                query: Box::new(bound_query),
                recursive: with.recursive,
            });
            self.cte_defs.push(cte_def.clone());
            ctes.push(cte_def);
        }

        ctes
    }

    pub(super) fn bind_subquery(&mut self, query: &Query) -> BoundQuery {
        let mut child = Binder::new(self.dialect, self.catalog);
        child.cte_scope = self.cte_scope.clone();
        child.cte_defs = self.cte_defs.clone();
        let bound = child
            .bind_query(query)
            .unwrap_or_else(|| child.empty_query());
        self.diagnostics.extend(child.diagnostics);
        bound
    }

    pub(super) fn bind_correlated_subquery(
        &mut self,
        query: &Query,
        outer_scope: &BindScope,
    ) -> BoundQuery {
        let mut child = Binder::new(self.dialect, self.catalog);
        child.cte_scope = self.cte_scope.clone();
        child.cte_defs = self.cte_defs.clone();

        let mut outer_scopes = Vec::new();
        outer_scopes.push(child.import_outer_scope(outer_scope, self));
        for scope in &self.outer_scopes {
            outer_scopes.push(child.import_outer_scope(scope, self));
        }
        child.outer_scopes = outer_scopes;

        let bound = child
            .bind_query(query)
            .unwrap_or_else(|| child.empty_query());
        self.diagnostics.extend(child.diagnostics);
        bound
    }

    pub(super) fn empty_query(&self) -> BoundQuery {
        BoundQuery {
            ctes: Vec::new(),
            tables: crate::ir::arena::Arena::default(),
            columns: crate::ir::arena::Arena::default(),
            exprs: crate::ir::arena::Arena::default(),
            body: BoundSetExpr::Unsupported,
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    fn import_outer_scope(&mut self, scope: &BindScope, outer: &Binder<'_>) -> BindScope {
        let mut merged = BindScope::default();
        for (alias, columns) in scope.tables() {
            let Some(first_col) = columns.first() else {
                continue;
            };
            let outer_col = outer.columns.get(first_col.id);
            let outer_table = outer.tables.get(outer_col.table).clone();
            let (_, table_scope) = self.register_table(outer_table, alias.to_string());
            merged.merge(table_scope);
        }
        merged
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

        let mut child = Binder::new(self.dialect, self.catalog);
        child.cte_scope = self.cte_scope.clone();
        let anchor_body = child.bind_set_expr(anchor_set);

        let anchor_query = BoundQuery {
            ctes: Vec::new(),
            tables: mem::take(&mut child.tables),
            columns: mem::take(&mut child.columns),
            exprs: mem::take(&mut child.exprs),
            body: anchor_body,
            order_by: Vec::new(),
            limit: None,
            offset: None,
        };

        self.diagnostics.extend(child.diagnostics);

        let mut names = self.output_names_for_query(&anchor_query);
        if !alias_columns.is_empty() {
            names = alias_columns.to_vec();
        }
        names
    }
}
