use std::collections::HashSet;

use sqlex_common::types::DataType;
use sqlparser::ast::{Expr, Select, SelectItem, SetExpr};

use crate::{
    algebra::{
        expr::{RelExpr, ScanNode},
        planner::{
            Algebraizer,
            context::{BuildContext, CteBinding},
        },
        scalar::{BoundColumn, ColumnOrigin, OutputSchema},
    },
    catalog::{model::Catalog, normalize::normalize_ident},
    diagnostics::{Diagnostic, Phase},
    functions::registry::FunctionRegistry,
};

impl Algebraizer {
    pub(crate) fn register_ctes(
        &self,
        with_clause: &sqlparser::ast::With,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &mut BuildContext,
    ) -> Result<(), Diagnostic> {
        let mut seen_names = HashSet::new();

        if with_clause.recursive {
            for cte in &with_clause.cte_tables {
                let cte_name = normalize_ident(&cte.alias.name, self.dialect);
                if !seen_names.insert(cte_name.clone()) || context.ctes.contains_key(&cte_name) {
                    return Err(Diagnostic::new(
                        "A3025",
                        Phase::Algebraize,
                        format!("duplicate CTE name: {cte_name}"),
                    ));
                }
                let binding = self.build_recursive_cte_stub(cte, catalog, context)?;
                context.ctes.insert(cte_name, binding);
            }
            return Ok(());
        }

        if matches!(self.dialect, sqlex_common::dialect::Dialect::SQLite) {
            for cte in &with_clause.cte_tables {
                if cte.from.is_some() {
                    return Err(Diagnostic::todo(
                        Phase::Algebraize,
                        "CTE SEARCH/CYCLE clause planning",
                    ));
                }

                let cte_name = normalize_ident(&cte.alias.name, self.dialect);
                if !seen_names.insert(cte_name.clone()) || context.ctes.contains_key(&cte_name) {
                    return Err(Diagnostic::new(
                        "A3025",
                        Phase::Algebraize,
                        format!("duplicate CTE name: {cte_name}"),
                    ));
                }
                let binding = self.build_recursive_cte_stub(cte, catalog, context)?;
                context.ctes.insert(cte_name, binding);
            }

            for _ in 0..with_clause.cte_tables.len() {
                for cte in &with_clause.cte_tables {
                    let cte_name = normalize_ident(&cte.alias.name, self.dialect);
                    let mut cte_context = BuildContext {
                        relation_scopes: Vec::new(),
                        outer_relation_scopes: context.outer_relation_scopes.clone(),
                        next_relation_id: context.next_relation_id,
                        next_slot_id: context.next_slot_id,
                        ctes: context.ctes.clone(),
                        named_windows: std::collections::HashMap::new(),
                        literal_assignment_mode: context.literal_assignment_mode,
                    };
                    let cte_expr =
                        self.build_set_expr(&cte.query.body, catalog, functions, &mut cte_context)?;
                    context.next_relation_id = cte_context.next_relation_id;
                    context.next_slot_id = cte_context.next_slot_id;

                    let mut exposed_schema = super::output_schema_of(&cte_expr)?;
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
                            column.name = normalize_ident(&alias_column.name, self.dialect);
                        }
                    }

                    context.ctes.insert(
                        cte_name.clone(),
                        CteBinding {
                            expr: cte_expr,
                            exposed_schema,
                        },
                    );
                }
            }
            return Ok(());
        }

        for cte in &with_clause.cte_tables {
            if cte.from.is_some() {
                return Err(Diagnostic::todo(
                    Phase::Algebraize,
                    "CTE SEARCH/CYCLE clause planning",
                ));
            }

            let cte_name = normalize_ident(&cte.alias.name, self.dialect);
            if !seen_names.insert(cte_name.clone()) || context.ctes.contains_key(&cte_name) {
                return Err(Diagnostic::new(
                    "A3025",
                    Phase::Algebraize,
                    format!("duplicate CTE name: {cte_name}"),
                ));
            }
            let mut cte_context = BuildContext {
                relation_scopes: Vec::new(),
                outer_relation_scopes: context.outer_relation_scopes.clone(),
                next_relation_id: context.next_relation_id,
                next_slot_id: context.next_slot_id,
                ctes: context.ctes.clone(),
                named_windows: std::collections::HashMap::new(),
                literal_assignment_mode: context.literal_assignment_mode,
            };
            let cte_expr =
                self.build_set_expr(&cte.query.body, catalog, functions, &mut cte_context)?;
            context.next_relation_id = cte_context.next_relation_id;
            context.next_slot_id = cte_context.next_slot_id;

            let mut exposed_schema = super::output_schema_of(&cte_expr)?;
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
                    column.name = normalize_ident(&alias_column.name, self.dialect);
                }
            }

            context.ctes.insert(
                cte_name,
                CteBinding {
                    expr: cte_expr,
                    exposed_schema,
                },
            );
        }

        Ok(())
    }

    fn build_recursive_cte_stub(
        &self,
        cte: &sqlparser::ast::Cte,
        catalog: &Catalog,
        context: &mut BuildContext,
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
            return Err(Diagnostic::todo(
                Phase::Algebraize,
                "recursive CTE seed extraction",
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
            let (data_type, nullable) = projection_expr(item)
                .and_then(|expr| self.resolve_scalar_subquery_expr_type(seed_select, expr, catalog))
                .unwrap_or((DataType::Custom("unknown".to_string()), true));

            let name = if !alias_columns.is_empty() {
                normalize_ident(&alias_columns[index].name, self.dialect)
            } else {
                match item {
                    SelectItem::ExprWithAlias { alias, .. } => normalize_ident(alias, self.dialect),
                    SelectItem::UnnamedExpr(expr) => self
                        .derive_output_name(expr)
                        .unwrap_or_else(|_| format!("column_{}", index + 1)),
                    SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _) => {
                        format!("column_{}", index + 1)
                    },
                }
            };

            columns.push(BoundColumn {
                slot_id: context.allocate_slot_id(),
                name,
                table_alias: None,
                data_type: Some(data_type),
                nullable,
                origin: ColumnOrigin::Derived,
            });
        }

        let cte_name = normalize_ident(&cte.alias.name, self.dialect);
        let schema = OutputSchema {
            relation_id: context.allocate_relation_id(),
            columns,
        };
        Ok(CteBinding {
            expr: RelExpr::Scan(ScanNode {
                table: format!("__recursive_cte__{cte_name}"),
                schema: schema.clone(),
            }),
            exposed_schema: schema,
        })
    }
}

fn recursive_cte_seed_select(set_expr: &SetExpr) -> Option<&Select> {
    match set_expr {
        SetExpr::Select(select) => Some(select),
        SetExpr::SetOperation { left, .. } => recursive_cte_seed_select(left),
        SetExpr::Query(query) => recursive_cte_seed_select(&query.body),
        _ => None,
    }
}

fn recursive_cte_set_operation_projection_counts(set_expr: &SetExpr) -> Option<(usize, usize)> {
    match set_expr {
        SetExpr::SetOperation { left, right, .. } => Some((
            recursive_cte_projection_count(left)?,
            recursive_cte_projection_count(right)?,
        )),
        SetExpr::Query(query) => recursive_cte_set_operation_projection_counts(&query.body),
        _ => None,
    }
}

fn recursive_cte_projection_count(set_expr: &SetExpr) -> Option<usize> {
    match set_expr {
        SetExpr::Select(select) => Some(select.projection.len()),
        SetExpr::SetOperation { left, .. } => recursive_cte_projection_count(left),
        SetExpr::Query(query) => recursive_cte_projection_count(&query.body),
        _ => None,
    }
}

fn projection_expr(item: &SelectItem) -> Option<&Expr> {
    match item {
        SelectItem::UnnamedExpr(expr) => Some(expr),
        SelectItem::ExprWithAlias { expr, .. } => Some(expr),
        SelectItem::QualifiedWildcard(_, _) | SelectItem::Wildcard(_) => None,
    }
}
