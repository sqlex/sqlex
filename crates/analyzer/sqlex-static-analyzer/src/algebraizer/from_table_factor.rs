use std::collections::HashSet;

use sqlparser::ast::TableFactor;

use crate::{
    algebraizer::{
        Algebraizer,
        model::{
            relation::{AliasNode, Relation, ScanNode},
            schema::{BoundColumn, ColumnOrigin, OutputSchema},
        },
        scope::RelationBinding,
    },
    catalog::{
        model::TableSchema,
        normalize::{normalize_ident, normalize_object_name},
    },
    diagnostics::{Diagnostic, Phase},
};

impl Algebraizer<'_> {
    pub(crate) fn build_table_factor(
        &mut self,
        relation: &TableFactor,
    ) -> Result<(Relation, RelationBinding), Diagnostic> {
        match relation {
            TableFactor::Table { name, alias, .. } => {
                if let Some(alias) = alias {
                    self.validate_alias_ident(&alias.name)?;
                }
                let normalized_table_name = normalize_object_name(name, self.dialect);
                if let Some(cte_binding) = self.resolve_cte(&normalized_table_name) {
                    let scope = RelationBinding {
                        qualifier_names: self
                            .qualifier_names_for_relation(&normalized_table_name, alias.as_ref()),
                        schema: cte_binding.exposed_schema.clone(),
                        hidden_unqualified_slot_ids: HashSet::new(),
                    };
                    return Ok((cte_binding.relation.clone(), scope));
                }

                let Some(table) = self.catalog.table(&normalized_table_name) else {
                    return Err(Diagnostic::new(
                        "A3003",
                        Phase::Algebraize,
                        format!("table not found: {normalized_table_name}"),
                    ));
                };

                let (schema, qualifier_names) =
                    self.build_table_scope(table, &normalized_table_name, alias.as_ref());
                let scope = RelationBinding {
                    qualifier_names,
                    schema: schema.clone(),
                    hidden_unqualified_slot_ids: HashSet::new(),
                };
                let scan_relation = Relation::Scan(ScanNode {
                    table: normalized_table_name,
                    schema,
                });
                let relation = if alias.is_some() {
                    Relation::Alias(AliasNode {
                        input: Box::new(scan_relation),
                        schema: scope.schema.clone(),
                    })
                } else {
                    scan_relation
                };
                Ok((relation, scope))
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

                let subquery_relation = self.build_query_relation(subquery, true)?;

                let Some(alias) = alias.as_ref() else {
                    return Err(Diagnostic::new(
                        "A3063",
                        Phase::Algebraize,
                        "derived table in FROM requires an alias in this iteration",
                    ));
                };
                self.validate_alias_ident(&alias.name)?;
                let alias_name = normalize_ident(&alias.name, self.dialect);

                let mut schema = super::output_schema_of(&subquery_relation)?;
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

                let scope = RelationBinding {
                    qualifier_names: vec![alias_name.clone()],
                    schema,
                    hidden_unqualified_slot_ids: HashSet::new(),
                };
                Ok((
                    Relation::Alias(AliasNode {
                        input: Box::new(subquery_relation),
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

    fn qualifier_names_for_relation(
        &self,
        normalized_table_name: &str,
        alias: Option<&sqlparser::ast::TableAlias>,
    ) -> Vec<String> {
        if let Some(alias) = alias {
            return vec![normalize_ident(&alias.name, self.dialect)];
        }

        let mut qualifier_names = Vec::new();
        qualifier_names.push(normalized_table_name.to_string());
        if let Some(last_segment) = normalized_table_name.split('.').next_back() {
            if !qualifier_names.iter().any(|name| name == last_segment) {
                qualifier_names.push(last_segment.to_string());
            }
        }
        qualifier_names
    }

    fn build_table_scope(
        &mut self,
        table: &TableSchema,
        normalized_table_name: &str,
        alias: Option<&sqlparser::ast::TableAlias>,
    ) -> (OutputSchema, Vec<String>) {
        let mut columns = Vec::with_capacity(table.columns.len());
        for column in &table.columns {
            let slot_id = self.allocate_slot_id();
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

        let qualifier_names = self.qualifier_names_for_relation(normalized_table_name, alias);

        (
            OutputSchema {
                relation_id: self.allocate_relation_id(),
                columns,
            },
            qualifier_names,
        )
    }
}
