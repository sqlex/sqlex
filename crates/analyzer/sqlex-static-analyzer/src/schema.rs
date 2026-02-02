//! Schema types for static SQL analysis
//!
//! Defines the data structures for representing database schema,
//! including tables, columns, constraints, and foreign keys.

use std::collections::HashMap;

use sqlex_common::DataType;

/// SQL dialect for parsing
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Dialect {
    #[default]
    PostgreSQL,
    MySQL,
    SQLite,
}

/// Database schema containing all table definitions
#[derive(Debug, Default)]
pub struct Schema {
    pub dialect: Dialect,
    pub tables: HashMap<String, TableDef>,
    /// Reverse index: target_table -> [(source_table, fk)]
    /// Used for quickly querying "who references this table"
    #[allow(dead_code)] // Will be used when FK lookup is implemented
    fk_reverse_index: HashMap<String, Vec<(String, ForeignKeyDef)>>,
}

impl Schema {
    /// Create a new empty schema with the given dialect
    pub fn new(dialect: Dialect) -> Self {
        Self {
            dialect,
            tables: HashMap::new(),
            fk_reverse_index: HashMap::new(),
        }
    }

    /// Rebuild the foreign key reverse index
    pub fn rebuild_fk_index(&mut self) {
        todo!("rebuild FK reverse index from all tables")
    }

    /// Get all foreign keys that reference a given table
    pub fn get_references_to(&self, _table: &str) -> Vec<&ForeignKeyDef> {
        todo!("lookup FK reverse index")
    }

    /// Get a table definition by name
    pub fn get_table(&self, name: &str) -> Option<&TableDef> {
        self.tables.get(name)
    }

    /// Add a table to the schema
    pub fn add_table(&mut self, table: TableDef) {
        self.tables.insert(table.name.clone(), table);
    }
}

/// Table definition
#[derive(Debug, Clone)]
pub struct TableDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub primary_key: Option<Vec<String>>,
    pub unique_constraints: Vec<Vec<String>>,
    /// Foreign keys from this table to other tables
    pub foreign_keys: Vec<ForeignKeyDef>,
}

impl TableDef {
    /// Create a new table definition
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            columns: Vec::new(),
            primary_key: None,
            unique_constraints: Vec::new(),
            foreign_keys: Vec::new(),
        }
    }

    /// Get a column by name
    pub fn get_column(&self, name: &str) -> Option<&ColumnDef> {
        self.columns.iter().find(|c| c.name == name)
    }
}

/// Column definition
#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default: Option<String>,
}

impl ColumnDef {
    /// Create a new column definition
    pub fn new(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable: true, // Default to nullable
            default: None,
        }
    }

    /// Set the column as NOT NULL
    pub fn not_null(mut self) -> Self {
        self.nullable = false;
        self
    }

    /// Set the default value
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }
}

/// Foreign key constraint
#[derive(Debug, Clone)]
pub struct ForeignKeyDef {
    pub name: Option<String>,
    pub columns: Vec<String>,
    pub ref_table: String,
    pub ref_columns: Vec<String>,
    pub on_delete: Option<ReferentialAction>,
    pub on_update: Option<ReferentialAction>,
}

impl ForeignKeyDef {
    /// Create a new foreign key definition
    pub fn new(
        columns: Vec<String>,
        ref_table: impl Into<String>,
        ref_columns: Vec<String>,
    ) -> Self {
        Self {
            name: None,
            columns,
            ref_table: ref_table.into(),
            ref_columns,
            on_delete: None,
            on_update: None,
        }
    }
}

/// Referential action for foreign key constraints
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferentialAction {
    Cascade,
    SetNull,
    SetDefault,
    Restrict,
    NoAction,
}
