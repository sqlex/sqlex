//! Catalog types for static SQL analysis
//!
//! Defines the data structures for representing database catalog metadata,
//! including tables, columns, constraints, and foreign keys.

use sqlex_common::types::DataType;

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
