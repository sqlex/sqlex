use std::collections::{HashMap, HashSet};

use crate::{analysis::diagnostics::Diagnostic, ir::ids::ColumnId};

#[derive(Debug, Clone, Default)]
pub(super) struct BindScope {
    tables: Vec<ScopeTable>,
    by_alias: HashMap<String, usize>,
}

#[derive(Debug, Clone)]
struct ScopeTable {
    alias: String,
    columns: Vec<ScopeColumn>,
}

#[derive(Debug, Clone)]
pub(super) struct ScopeColumn {
    pub(super) name: String,
    pub(super) id: ColumnId,
}

impl BindScope {
    pub(super) fn add_table(&mut self, alias: String, columns: Vec<ScopeColumn>) {
        let index = self.tables.len();
        self.tables.push(ScopeTable {
            alias: alias.clone(),
            columns,
        });
        self.by_alias.insert(alias, index);
    }

    pub(super) fn merge(&mut self, other: BindScope) {
        for table in other.tables {
            self.add_table(table.alias, table.columns);
        }
    }

    pub(super) fn resolve_column(
        &self,
        table_alias: Option<&str>,
        column: &str,
    ) -> Result<ColumnId, Diagnostic> {
        if let Some(alias) = table_alias {
            let index = self
                .by_alias
                .get(alias)
                .ok_or_else(|| Diagnostic::unknown_table_alias(alias))?;
            let table = &self.tables[*index];
            table
                .columns
                .iter()
                .find(|c| c.name == column)
                .map(|c| c.id)
                .ok_or_else(|| Diagnostic::unknown_column(&format!("{alias}.{column}")))
        } else {
            let mut found = None;
            for table in &self.tables {
                for col in &table.columns {
                    if col.name == column {
                        if found.is_some() {
                            return Err(Diagnostic::ambiguous_column(column));
                        }
                        found = Some(col.id);
                    }
                }
            }

            found.ok_or_else(|| Diagnostic::unknown_column(column))
        }
    }

    pub(super) fn columns_in_order(&self) -> Vec<ScopeColumn> {
        let mut cols = Vec::new();
        for table in &self.tables {
            cols.extend(table.columns.iter().cloned());
        }
        cols
    }

    pub(super) fn columns_for_table(&self, alias: &str) -> Option<Vec<ScopeColumn>> {
        self.by_alias
            .get(alias)
            .map(|idx| self.tables[*idx].columns.clone())
    }

    pub(super) fn tables(&self) -> impl Iterator<Item = (&str, &[ScopeColumn])> {
        self.tables
            .iter()
            .map(|table| (table.alias.as_str(), table.columns.as_slice()))
    }

    pub(super) fn column_names_set(&self) -> HashSet<String> {
        let mut set = HashSet::new();
        for table in &self.tables {
            for col in &table.columns {
                set.insert(col.name.clone());
            }
        }
        set
    }

    pub(super) fn has_column(&self, name: &str) -> bool {
        self.tables
            .iter()
            .any(|table| table.columns.iter().any(|col| col.name == name))
    }

    pub(super) fn drop_columns(&mut self, names: &HashSet<String>) {
        for table in &mut self.tables {
            table.columns.retain(|col| !names.contains(&col.name));
        }
    }
}
