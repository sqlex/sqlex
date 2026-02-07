use std::collections::HashSet;

use sqlparser::ast::{JoinConstraint, JoinOperator, TableFactor, TableWithJoins};

use crate::{
    analysis::{
        bind::{
            Binder,
            scope::{BindScope, ScopeColumn},
        },
        diagnostics::{Diagnostic, DiagnosticCode},
        keywords,
    },
    ir::{
        bound::{
            BoundColumn, BoundFromItem, BoundJoin, BoundJoinCondition, BoundJoinKind, BoundTable,
            BoundTableSource,
        },
        ids::TableId,
    },
};

impl<'a> Binder<'a> {
    pub(super) fn bind_from(&mut self, from: &[TableWithJoins]) -> (Vec<BoundFromItem>, BindScope) {
        let mut scope = BindScope::default();
        let mut items = Vec::new();

        for item in from {
            let (from_item, item_scope) = self.bind_table_with_joins(item, &scope);
            scope.merge(item_scope);
            items.push(from_item);
        }

        (items, scope)
    }

    fn bind_table_with_joins(
        &mut self,
        table_with_joins: &TableWithJoins,
        scope: &BindScope,
    ) -> (BoundFromItem, BindScope) {
        let (table_id, mut local_scope) = self.bind_table_factor(&table_with_joins.relation, scope);
        let mut from_item = BoundFromItem {
            table: table_id,
            joins: Vec::new(),
        };

        for join in &table_with_joins.joins {
            let (right_id, mut right_scope) = self.bind_table_factor(&join.relation, &local_scope);

            let (kind, condition) =
                self.bind_join_operator(&join.join_operator, &local_scope, &right_scope);
            let using_columns = self.using_columns_for_join(&condition, &local_scope, &right_scope);
            from_item.joins.push(BoundJoin {
                kind,
                table: right_id,
                condition: condition.clone(),
            });

            if let Some(using_columns) = using_columns {
                let using_set: HashSet<String> = using_columns.into_iter().collect();
                right_scope.drop_columns(&using_set);
            }

            local_scope.merge(right_scope);
        }

        (from_item, local_scope)
    }

    fn bind_join_operator(
        &mut self,
        join_operator: &JoinOperator,
        left_scope: &BindScope,
        right_scope: &BindScope,
    ) -> (BoundJoinKind, Option<BoundJoinCondition>) {
        let (kind, constraint) = match join_operator {
            JoinOperator::Inner(constraint) => (BoundJoinKind::Inner, Some(constraint)),
            JoinOperator::LeftOuter(constraint) => (BoundJoinKind::Left, Some(constraint)),
            JoinOperator::RightOuter(constraint) => (BoundJoinKind::Right, Some(constraint)),
            JoinOperator::FullOuter(constraint) => {
                if self.dialect == sqlex_common::dialect::Dialect::MySQL {
                    self.diagnostics.push(Diagnostic::error_with_code(
                        DiagnosticCode::InvalidJoin,
                        "FULL JOIN is not supported in MySQL",
                    ));
                }
                (BoundJoinKind::Full, Some(constraint))
            },
            JoinOperator::CrossJoin => (BoundJoinKind::Cross, None),
            _ => {
                self.diagnostics
                    .push(Diagnostic::unsupported_feature("join type in binder"));
                (BoundJoinKind::Inner, None)
            },
        };

        let condition = constraint.and_then(|constraint| {
            let mut combined = left_scope.clone();
            combined.merge(right_scope.clone());

            Some(match constraint {
                JoinConstraint::On(expr) => {
                    let expr_id = self.bind_expr(expr, &combined);
                    BoundJoinCondition::On(expr_id)
                },
                JoinConstraint::Using(names) => {
                    let columns = names
                        .iter()
                        .map(|name| {
                            name.0
                                .last()
                                .map(|ident| ident.value.clone())
                                .unwrap_or_else(|| name.to_string())
                        })
                        .collect();
                    BoundJoinCondition::Using(columns)
                },
                JoinConstraint::Natural => BoundJoinCondition::Natural,
                JoinConstraint::None => {
                    self.diagnostics
                        .push(Diagnostic::unsupported_feature("JOIN constraint NONE"));
                    return None;
                },
            })
        });

        (kind, condition)
    }

    fn bind_table_factor(
        &mut self,
        table: &TableFactor,
        _scope: &BindScope,
    ) -> (TableId, BindScope) {
        match table {
            TableFactor::Table { name, alias, .. } => {
                let table_name = name.to_string();
                let alias_name = alias.as_ref().map(|a| a.name.value.clone());
                if let Some(alias) = alias.as_ref() {
                    if keywords::is_reserved_identifier(self.dialect, &alias.name) {
                        self.diagnostics.push(Diagnostic::invalid_statement(format!(
                            "Table alias {alias} is a reserved keyword in {dialect}; quote it to use as an identifier",
                            dialect = self.dialect
                        )));
                    }
                }

                if let Some(cte_binding) = self.cte_scope.get(&table_name) {
                    let columns = cte_binding.columns.clone();
                    let (table_id, scope) = self.register_table(
                        BoundTable {
                            source: BoundTableSource::Cte {
                                name: table_name.clone(),
                            },
                            alias: alias_name.clone(),
                            columns,
                        },
                        alias_name.unwrap_or_else(|| table_name.clone()),
                        None,
                    );
                    return (table_id, scope);
                }

                if let Some(table_def) = self.catalog.get_table(&table_name) {
                    let column_names = table_def.columns.iter().map(|c| c.name.clone()).collect();
                    let (table_id, scope) = self.register_table(
                        BoundTable {
                            source: BoundTableSource::Table {
                                name: table_name.clone(),
                            },
                            alias: alias_name.clone(),
                            columns: column_names,
                        },
                        alias_name.unwrap_or_else(|| table_name.clone()),
                        Some(table_def),
                    );
                    (table_id, scope)
                } else {
                    self.diagnostics
                        .push(Diagnostic::unknown_table(&table_name));
                    let (table_id, scope) = self.register_table(
                        BoundTable {
                            source: BoundTableSource::Table {
                                name: table_name.clone(),
                            },
                            alias: alias_name.clone(),
                            columns: Vec::new(),
                        },
                        alias_name.unwrap_or(table_name),
                        None,
                    );
                    (table_id, scope)
                }
            },
            TableFactor::Derived {
                subquery, alias, ..
            } => self.bind_derived_table(subquery, alias.as_ref()),
            _ => {
                self.diagnostics
                    .push(Diagnostic::unsupported_feature("table factor in binder"));

                let (table_id, scope) = self.register_table(
                    BoundTable {
                        source: BoundTableSource::Table {
                            name: "<unknown>".to_string(),
                        },
                        alias: None,
                        columns: Vec::new(),
                    },
                    "<unknown>".to_string(),
                    None,
                );

                (table_id, scope)
            },
        }
    }

    fn bind_derived_table(
        &mut self,
        subquery: &sqlparser::ast::Query,
        alias: Option<&sqlparser::ast::TableAlias>,
    ) -> (TableId, BindScope) {
        let alias_name = alias.map(|a| a.name.value.clone());
        let Some(alias_str) = alias_name.clone() else {
            self.diagnostics
                .push(Diagnostic::derived_table_requires_alias());
            let bound_query = self.bind_query_body(subquery);
            let (table_id, scope) = self.register_table(
                BoundTable {
                    source: BoundTableSource::Derived { query: bound_query },
                    alias: None,
                    columns: Vec::new(),
                },
                "<derived>".to_string(),
                None,
            );
            return (table_id, scope);
        };

        if let Some(a) = alias {
            if keywords::is_reserved_identifier(self.dialect, &a.name) {
                self.diagnostics.push(Diagnostic::invalid_statement(format!(
                    "Table alias {a} is a reserved keyword in {dialect}; quote it to use as an identifier",
                    dialect = self.dialect
                )));
            }
            for column_alias in &a.columns {
                if keywords::is_reserved_identifier(self.dialect, &column_alias.name) {
                    self.diagnostics.push(Diagnostic::invalid_statement(format!(
                        "Derived column alias {alias} is a reserved keyword in {dialect}; quote it to use as an identifier",
                        alias = column_alias.name,
                        dialect = self.dialect
                    )));
                }
            }
        }

        let bound_query = self.bind_query_body(subquery);
        let column_aliases = alias
            .map(|a| {
                a.columns
                    .iter()
                    .map(|c| c.name.value.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let column_names = if !column_aliases.is_empty() {
            column_aliases
        } else {
            self.output_names_for_query_body(&bound_query)
        };

        let (table_id, scope) = self.register_table(
            BoundTable {
                source: BoundTableSource::Derived { query: bound_query },
                alias: Some(alias_str.clone()),
                columns: column_names,
            },
            alias_str,
            None,
        );

        (table_id, scope)
    }

    pub(super) fn register_table(
        &mut self,
        table: BoundTable,
        alias: String,
        table_def: Option<&crate::catalog::types::TableDef>,
    ) -> (TableId, BindScope) {
        let column_names = table.columns.clone();
        let table_id = self.tables.alloc(table);
        let mut scope = BindScope::default();
        let mut cols = Vec::new();
        for col_name in &column_names {
            let (data_type, nullable) = table_def
                .and_then(|td| td.get_column(col_name))
                .map(|cd| (Some(cd.data_type.clone()), Some(cd.nullable)))
                .unwrap_or((None, None));
            let col_id = self.columns.alloc(BoundColumn {
                table: table_id,
                name: col_name.clone(),
                data_type,
                nullable,
            });
            cols.push(ScopeColumn {
                name: col_name.clone(),
                id: col_id,
            });
        }
        scope.add_table(alias, cols);
        (table_id, scope)
    }

    fn using_columns_for_join(
        &mut self,
        condition: &Option<BoundJoinCondition>,
        left_scope: &BindScope,
        right_scope: &BindScope,
    ) -> Option<Vec<String>> {
        let columns = match condition {
            Some(BoundJoinCondition::Using(cols)) => cols.clone(),
            Some(BoundJoinCondition::Natural) => {
                let left = left_scope.column_names_set();
                let right = right_scope.column_names_set();
                left.intersection(&right).cloned().collect()
            },
            _ => return None,
        };

        for col in &columns {
            if !left_scope.has_column(col) || !right_scope.has_column(col) {
                self.diagnostics
                    .push(Diagnostic::join_using_column_missing(col));
            }
        }

        Some(columns)
    }
}
