use std::collections::HashSet;

use sqlparser::ast::TableFactor;

use crate::{
    algebraizer::{
        Algebraizer,
        context::{BuildContext, RelationScope},
        model::{
            relation::{AliasNode, Relation, ScanNode},
            schema::{BoundColumn, ColumnOrigin, OutputSchema},
        },
    },
    catalog::{
        Catalog,
        model::TableSchema,
        normalize::{normalize_ident, normalize_object_name},
    },
    diagnostics::{Diagnostic, Phase},
    functions::FunctionRegistry,
};

impl Algebraizer {
    pub(crate) fn build_table_factor(
        &self,
        relation: &TableFactor,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &mut BuildContext,
    ) -> Result<(Relation, RelationScope), Diagnostic> {
        match relation {
            TableFactor::Table { name, alias, .. } => {
                if let Some(alias) = alias {
                    self.validate_alias_ident(&alias.name)?;
                }
                let normalized_table_name = normalize_object_name(name, self.dialect);
                if let Some(cte_binding) = context.ctes.get(&normalized_table_name) {
                    let scope = RelationScope {
                        visible_names: self
                            .visible_names_for_relation(&normalized_table_name, alias.as_ref()),
                        schema: cte_binding.exposed_schema.clone(),
                        hidden_unqualified_slots: HashSet::new(),
                    };
                    return Ok((cte_binding.expr.clone(), scope));
                }

                let Some(table) = catalog.table(&normalized_table_name) else {
                    return Err(Diagnostic::new(
                        "A3003",
                        Phase::Algebraize,
                        format!("table not found: {normalized_table_name}"),
                    ));
                };

                let (schema, visible_names) =
                    self.build_table_scope(table, &normalized_table_name, alias.as_ref(), context);
                let scope = RelationScope {
                    visible_names,
                    schema: schema.clone(),
                    hidden_unqualified_slots: HashSet::new(),
                };
                let scan_expr = Relation::Scan(ScanNode {
                    table: normalized_table_name,
                    schema,
                });
                let relation_expr = if alias.is_some() {
                    Relation::Alias(AliasNode {
                        input: Box::new(scan_expr),
                        schema: scope.schema.clone(),
                    })
                } else {
                    scan_expr
                };
                Ok((relation_expr, scope))
            },
            TableFactor::Derived {
                lateral,
                subquery,
                alias,
            } => {
                if *lateral {
                    return Err(Diagnostic::new(
                        "A3062",
                        Phase::Algebraize,
                        "LATERAL derived tables are not supported in this iteration",
                    ));
                }

                let mut subquery_context = BuildContext {
                    relation_scopes: context.relation_scopes.clone(),
                    outer_relation_scopes: context.outer_relation_scopes.clone(),
                    next_relation_id: context.next_relation_id,
                    next_slot_id: context.next_slot_id,
                    ctes: context.ctes.clone(),
                    named_windows: std::collections::HashMap::new(),
                    literal_assignment_mode: true,
                };
                let subquery_expr =
                    self.build_set_expr(&subquery.body, catalog, functions, &mut subquery_context)?;
                context.next_relation_id = subquery_context.next_relation_id;
                context.next_slot_id = subquery_context.next_slot_id;

                let Some(alias) = alias.as_ref() else {
                    return Err(Diagnostic::new(
                        "A3063",
                        Phase::Algebraize,
                        "derived table in FROM requires an alias in this iteration",
                    ));
                };
                self.validate_alias_ident(&alias.name)?;
                let alias_name = normalize_ident(&alias.name, self.dialect);

                let mut schema = super::output_schema_of(&subquery_expr)?;
                if !alias.columns.is_empty() {
                    if alias.columns.len() != schema.columns.len() {
                        return Err(Diagnostic::new(
                            "A3013",
                            Phase::Algebraize,
                            format!(
                                "derived table alias column count mismatch: expected {}, got {}",
                                schema.columns.len(),
                                alias.columns.len()
                            ),
                        ));
                    }

                    for (column, alias_column) in
                        schema.columns.iter_mut().zip(alias.columns.iter())
                    {
                        self.validate_alias_ident(&alias_column.name)?;
                        column.name = normalize_ident(&alias_column.name, self.dialect);
                    }
                }

                let scope = RelationScope {
                    visible_names: vec![alias_name.clone()],
                    schema,
                    hidden_unqualified_slots: HashSet::new(),
                };
                Ok((
                    Relation::Alias(AliasNode {
                        input: Box::new(subquery_expr),
                        schema: scope.schema.clone(),
                    }),
                    scope,
                ))
            },
            _ => Err(Diagnostic::new(
                "A3064",
                Phase::Algebraize,
                format!("unsupported table factor in this iteration: {relation}"),
            )),
        }
    }

    fn visible_names_for_relation(
        &self,
        normalized_table_name: &str,
        alias: Option<&sqlparser::ast::TableAlias>,
    ) -> Vec<String> {
        if let Some(alias) = alias {
            return vec![normalize_ident(&alias.name, self.dialect)];
        }

        let mut visible_names = Vec::new();
        visible_names.push(normalized_table_name.to_string());
        if let Some(last_segment) = normalized_table_name.split('.').next_back() {
            if !visible_names.iter().any(|name| name == last_segment) {
                visible_names.push(last_segment.to_string());
            }
        }
        visible_names
    }

    fn build_table_scope(
        &self,
        table: &TableSchema,
        normalized_table_name: &str,
        alias: Option<&sqlparser::ast::TableAlias>,
        context: &mut BuildContext,
    ) -> (OutputSchema, Vec<String>) {
        let mut columns = Vec::with_capacity(table.columns.len());
        for column in &table.columns {
            let slot_id = context.allocate_slot_id();
            columns.push(BoundColumn {
                slot_id,
                name: column.name.clone(),
                table_alias: alias
                    .as_ref()
                    .map(|table_alias| normalize_ident(&table_alias.name, self.dialect))
                    .or_else(|| {
                        normalized_table_name
                            .split('.')
                            .next_back()
                            .map(|name| name.to_string())
                    }),
                data_type: Some(column.data_type.clone()),
                nullable: column.nullable,
                origin: ColumnOrigin::Base {
                    table: normalized_table_name.to_string(),
                    column: column.name.clone(),
                },
            });
        }

        let visible_names = self.visible_names_for_relation(normalized_table_name, alias);

        (
            OutputSchema {
                relation_id: context.allocate_relation_id(),
                columns,
            },
            visible_names,
        )
    }
}
