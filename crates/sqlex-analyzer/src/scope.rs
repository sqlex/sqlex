//! Query scope for name resolution.

use std::collections::HashMap;

use sqlex_types::{ColumnDef, SqlType, TableDef};

/// Information about a table in scope.
#[derive(Debug, Clone)]
pub struct ScopeTable {
    /// Alias or table name used to reference this table
    pub alias: String,
    /// Original table name
    pub table_name: String,
    /// Column definitions
    pub columns: Vec<ColumnDef>,
    /// Whether this table's columns are nullable due to outer join
    pub nullable_from_join: bool,
}

impl ScopeTable {
    /// Create a scope table from a table definition.
    pub fn from_table_def(table: &TableDef, alias: Option<&str>) -> Self {
        Self {
            alias: alias.unwrap_or(&table.name).to_string(),
            table_name: table.name.clone(),
            columns: table.columns.clone(),
            nullable_from_join: false,
        }
    }

    /// Get a column by name.
    pub fn get_column(&self, name: &str) -> Option<&ColumnDef> {
        self.columns
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(name))
    }
}

/// A scope containing tables available for column resolution.
#[derive(Debug, Clone, Default)]
pub struct Scope<'a> {
    /// Tables in scope, keyed by alias (lowercase)
    tables: HashMap<String, ScopeTable>,
    /// Order of table aliases for resolving unqualified columns
    table_order: Vec<String>,
    /// Parent scope (for outer queries/lateral joins)
    parent: Option<&'a Scope<'a>>,
}

impl<'a> Scope<'a> {
    /// Create a new empty scope.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a child scope with a parent.
    pub fn new_child(parent: &'a Scope<'a>) -> Self {
        Self {
            tables: HashMap::new(),
            table_order: Vec::new(),
            parent: Some(parent),
        }
    }

    /// Add a table to the scope.
    pub fn add_table(&mut self, scope_table: ScopeTable) {
        let key = scope_table.alias.to_lowercase();
        self.table_order.push(key.clone());
        self.tables.insert(key, scope_table);
    }

    /// Resolve a column reference.
    /// Returns (table_alias, column_def, is_nullable).
    pub fn resolve_column(
        &self,
        table_alias: Option<&str>,
        column_name: &str,
    ) -> Result<ResolvedColumn, ColumnResolutionError> {
        if let Some(alias) = table_alias {
            // Qualified column reference
            if let Some(table) = self.tables.get(&alias.to_lowercase()) {
                if let Some(column) = table.get_column(column_name) {
                    return Ok(ResolvedColumn {
                        _table_alias: table.alias.clone(),
                        table_name: table.table_name.clone(),
                        column_name: column.name.clone(),
                        data_type: column.data_type.clone(),
                        nullable: column.nullable || table.nullable_from_join,
                    });
                } else {
                    // Table found but column not found -> Error
                    return Err(ColumnResolutionError::UnknownColumn(
                        column_name.to_string(),
                    ));
                }
            }

            // Not found in this scope, check parent
            if let Some(parent) = self.parent {
                return parent.resolve_column(table_alias, column_name);
            }

            Err(ColumnResolutionError::UnknownTable(alias.to_string()))
        } else {
            // Unqualified column reference - search all tables
            let mut found: Option<ResolvedColumn> = None;

            for alias in &self.table_order {
                let table = self.tables.get(alias).unwrap();
                if let Some(column) = table.get_column(column_name) {
                    if found.is_some() {
                        return Err(ColumnResolutionError::Ambiguous(column_name.to_string()));
                    }
                    found = Some(ResolvedColumn {
                        _table_alias: table.alias.clone(),
                        table_name: table.table_name.clone(),
                        column_name: column.name.clone(),
                        data_type: column.data_type.clone(),
                        nullable: column.nullable || table.nullable_from_join,
                    });
                }
            }

            if let Some(resolved) = found {
                return Ok(resolved);
            }

            // Not found in this scope, check parent
            if let Some(parent) = self.parent {
                return parent.resolve_column(None, column_name);
            }

            Err(ColumnResolutionError::UnknownColumn(
                column_name.to_string(),
            ))
        }
    }

    /// Get all columns from all tables (for SELECT *).
    pub fn all_columns(&self) -> Vec<ResolvedColumn> {
        let mut columns = Vec::new();
        // Add columns from this scope
        for alias in &self.table_order {
            let table = self.tables.get(alias).unwrap();
            for col in &table.columns {
                columns.push(ResolvedColumn {
                    _table_alias: table.alias.clone(),
                    table_name: table.table_name.clone(),
                    column_name: col.name.clone(),
                    data_type: col.data_type.clone(),
                    nullable: col.nullable || table.nullable_from_join,
                });
            }
        }

        // Note: SELECT * typically only considers the current query level's FROM clause,
        // it does NOT include columns from the outer query (parent scope).
        // So we do NOT recurse to parent here.

        columns
    }

    /// Get all columns from a specific table (for SELECT table.*).
    pub fn table_columns(&self, alias: &str) -> Option<Vec<ResolvedColumn>> {
        // Here we recurse because we look for the table
        if let Some(table) = self.tables.get(&alias.to_lowercase()) {
            Some(
                table
                    .columns
                    .iter()
                    .map(|col| ResolvedColumn {
                        _table_alias: table.alias.clone(),
                        table_name: table.table_name.clone(),
                        column_name: col.name.clone(),
                        data_type: col.data_type.clone(),
                        nullable: col.nullable || table.nullable_from_join,
                    })
                    .collect(),
            )
        } else if let Some(parent) = self.parent {
            parent.table_columns(alias)
        } else {
            None
        }
    }
}

/// A resolved column reference.
#[derive(Debug, Clone)]
pub struct ResolvedColumn {
    /// Table alias
    pub _table_alias: String,
    /// Original table name
    pub table_name: String,
    /// Column name
    pub column_name: String,
    /// Column data type
    pub data_type: SqlType,
    /// Whether the column is nullable
    pub nullable: bool,
}

/// Errors during column resolution.
#[derive(Debug)]
#[allow(dead_code)]
pub enum ColumnResolutionError {
    UnknownTable(String),
    UnknownColumn(String),
    Ambiguous(String),
}
