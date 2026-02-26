use std::collections::HashSet;

use sqlex_analyzer::extension::ident_ext::IdentExt;
use sqlex_common::types::DataType;
use sqlparser::ast::{Expr, Select, SelectItem, SetExpr};

use crate::{
    algebraizer::{
        Algebraizer,
        model::{
            relation::{Relation, ScanNode},
            schema::{BoundColumn, ColumnOrigin, OutputSchema},
        },
        scope::CteBinding,
    },
    diagnostics::{Diagnostic, Phase},
};

impl Algebraizer<'_> {
    pub(crate) fn register_ctes(
        &mut self,
        with_clause: &sqlparser::ast::With,
    ) -> Result<(), Diagnostic> {
        let mut seen_names = HashSet::new();

        if with_clause.recursive {
            for cte in &with_clause.cte_tables {
                let cte_name = cte.alias.name.to_normalized_string(self.dialect);
                if !seen_names.insert(cte_name.clone())
                    || self.cte_scope.exists_in_any_scope(&cte_name)
                {
                    return Err(Diagnostic::new(
                        "A3025",
                        Phase::Algebraize,
                        format!("duplicate CTE name: {cte_name}"),
                    ));
                }
                let binding = self.build_recursive_cte_stub(cte)?;
                self.cte_scope.register(cte_name, binding);
            }
            return Ok(());
        }

        if matches!(self.dialect, sqlex_common::dialect::Dialect::SQLite) {
            for cte in &with_clause.cte_tables {
                if cte.from.is_some() {
                    return Err(Diagnostic::new(
                        "A3066",
                        Phase::Algebraize,
                        "CTE SEARCH/CYCLE clauses are not supported in this iteration",
                    ));
                }

                let cte_name = cte.alias.name.to_normalized_string(self.dialect);
                if !seen_names.insert(cte_name.clone())
                    || self.cte_scope.exists_in_any_scope(&cte_name)
                {
                    return Err(Diagnostic::new(
                        "A3025",
                        Phase::Algebraize,
                        format!("duplicate CTE name: {cte_name}"),
                    ));
                }
                let binding = self.build_recursive_cte_stub(cte)?;
                self.cte_scope.register(cte_name, binding);
            }

            for _ in 0..with_clause.cte_tables.len() {
                for cte in &with_clause.cte_tables {
                    let cte_name = cte.alias.name.to_normalized_string(self.dialect);
                    let cte_relation = self.build_query_relation(cte.query.as_ref())?;

                    let mut exposed_schema = cte_relation.output_schema().clone();
                    if !cte.alias.columns.is_empty() {
                        if cte.alias.columns.len() != exposed_schema.columns.len() {
                            return Err(Diagnostic::new(
                                "A3014",
                                Phase::Algebraize,
                                format!(
                                    "CTE column alias count mismatch: expected {}, got {}",
                                    exposed_schema.columns.len(),
                                    cte.alias.columns.len()
                                ),
                            ));
                        }
                        for (column, alias_column) in exposed_schema
                            .columns
                            .iter_mut()
                            .zip(cte.alias.columns.iter())
                        {
                            column.name = alias_column.name.to_normalized_string(self.dialect);
                        }
                    }

                    self.cte_scope.register(
                        cte_name.clone(),
                        CteBinding {
                            relation: cte_relation,
                            exposed_schema,
                        },
                    );
                }
            }
            return Ok(());
        }

        for cte in &with_clause.cte_tables {
            if cte.from.is_some() {
                return Err(Diagnostic::new(
                    "A3066",
                    Phase::Algebraize,
                    "CTE SEARCH/CYCLE clauses are not supported in this iteration",
                ));
            }

            let cte_name = cte.alias.name.to_normalized_string(self.dialect);
            if !seen_names.insert(cte_name.clone()) || self.cte_scope.exists_in_any_scope(&cte_name)
            {
                return Err(Diagnostic::new(
                    "A3025",
                    Phase::Algebraize,
                    format!("duplicate CTE name: {cte_name}"),
                ));
            }
            let cte_relation = self.build_query_relation(cte.query.as_ref())?;

            let mut exposed_schema = cte_relation.output_schema().clone();
            if !cte.alias.columns.is_empty() {
                if cte.alias.columns.len() != exposed_schema.columns.len() {
                    return Err(Diagnostic::new(
                        "A3014",
                        Phase::Algebraize,
                        format!(
                            "CTE column alias count mismatch: expected {}, got {}",
                            exposed_schema.columns.len(),
                            cte.alias.columns.len()
                        ),
                    ));
                }
                for (column, alias_column) in exposed_schema
                    .columns
                    .iter_mut()
                    .zip(cte.alias.columns.iter())
                {
                    column.name = alias_column.name.to_normalized_string(self.dialect);
                }
            }

            self.cte_scope.register(
                cte_name,
                CteBinding {
                    relation: cte_relation,
                    exposed_schema,
                },
            );
        }

        Ok(())
    }

    fn build_recursive_cte_stub(
        &mut self,
        cte: &sqlparser::ast::Cte,
    ) -> Result<CteBinding, Diagnostic> {
        if let Some((seed_count, recursive_count)) =
            recursive_cte_set_operation_projection_counts(&cte.query.body)
        {
            if seed_count != recursive_count {
                return Err(Diagnostic::new(
                    "A3026",
                    Phase::Algebraize,
                    format!(
                        "recursive CTE term column count mismatch: seed {}, recursive {}",
                        seed_count, recursive_count
                    ),
                ));
            }
        }

        let Some(seed_select) = recursive_cte_seed_select(&cte.query.body) else {
            return Err(Diagnostic::new(
                "A3067",
                Phase::Algebraize,
                "recursive CTE seed term must be SELECT-compatible in this iteration",
            ));
        };

        let alias_columns = &cte.alias.columns;
        if !alias_columns.is_empty() && alias_columns.len() != seed_select.projection.len() {
            return Err(Diagnostic::new(
                "A3015",
                Phase::Algebraize,
                format!(
                    "recursive CTE column alias count mismatch: expected {}, got {}",
                    seed_select.projection.len(),
                    alias_columns.len()
                ),
            ));
        }

        let mut columns = Vec::with_capacity(seed_select.projection.len());
        for (index, item) in seed_select.projection.iter().enumerate() {
            let (data_type, nullable) = projection_sql_expr(item)
                .and_then(|expr| self.resolve_scalar_subquery_expr_type(seed_select, expr))
                .unwrap_or((DataType::Custom("unknown".to_string()), true));

            let name = if !alias_columns.is_empty() {
                alias_columns[index].name.to_normalized_string(self.dialect)
            } else {
                match item {
                    SelectItem::ExprWithAlias { alias, .. } => {
                        alias.to_normalized_string(self.dialect)
                    },
                    SelectItem::UnnamedExpr(expr) => self
                        .derive_output_name(expr)
                        .unwrap_or_else(|_| format!("column_{}", index + 1)),
                    SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _) => {
                        format!("column_{}", index + 1)
                    },
                }
            };

            columns.push(BoundColumn {
                slot_id: self.allocate_slot_id(),
                name,
                table_alias: None,
                data_type: Some(data_type),
                nullable,
                origin: ColumnOrigin::Derived,
            });
        }

        let cte_name = cte.alias.name.to_normalized_string(self.dialect);
        let schema = OutputSchema {
            relation_id: self.allocate_relation_id(),
            columns,
        };
        Ok(CteBinding {
            relation: Relation::Scan(ScanNode {
                table: format!("__recursive_cte__{cte_name}"),
                schema: schema.clone(),
            }),
            exposed_schema: schema,
        })
    }
}

fn recursive_cte_seed_select(sql_set_expr: &SetExpr) -> Option<&Select> {
    match sql_set_expr {
        SetExpr::Select(select) => Some(select),
        SetExpr::SetOperation { left, .. } => recursive_cte_seed_select(left),
        SetExpr::Query(query) => recursive_cte_seed_select(&query.body),
        _ => None,
    }
}

fn recursive_cte_set_operation_projection_counts(sql_set_expr: &SetExpr) -> Option<(usize, usize)> {
    match sql_set_expr {
        SetExpr::SetOperation { left, right, .. } => Some((
            recursive_cte_projection_count(left)?,
            recursive_cte_projection_count(right)?,
        )),
        SetExpr::Query(query) => recursive_cte_set_operation_projection_counts(&query.body),
        _ => None,
    }
}

fn recursive_cte_projection_count(sql_set_expr: &SetExpr) -> Option<usize> {
    match sql_set_expr {
        SetExpr::Select(select) => Some(select.projection.len()),
        SetExpr::SetOperation { left, .. } => recursive_cte_projection_count(left),
        SetExpr::Query(query) => recursive_cte_projection_count(&query.body),
        _ => None,
    }
}

fn projection_sql_expr(item: &SelectItem) -> Option<&Expr> {
    match item {
        SelectItem::UnnamedExpr(expr) => Some(expr),
        SelectItem::ExprWithAlias { expr, .. } => Some(expr),
        SelectItem::QualifiedWildcard(_, _) | SelectItem::Wildcard(_) => None,
    }
}
