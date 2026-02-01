//! Table and column definitions.

use crate::SqlType;

/// Column definition within a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDef {
    /// Column name
    pub name: String,
    /// Column data type
    pub data_type: SqlType,
    /// Whether the column allows NULL values
    pub nullable: bool,
    /// Default value expression (as SQL string)
    pub default: Option<String>,
    /// Whether this column is part of the primary key
    pub is_primary_key: bool,
}

impl ColumnDef {
    /// Create a new column definition.
    pub fn new(name: impl Into<String>, data_type: SqlType) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable: true,
            default: None,
            is_primary_key: false,
        }
    }

    /// Set the column as NOT NULL.
    pub fn not_null(mut self) -> Self {
        self.nullable = false;
        self
    }

    /// Set the default value.
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Set as primary key.
    pub fn primary_key(mut self) -> Self {
        self.is_primary_key = true;
        self.nullable = false; // Primary keys are implicitly NOT NULL
        self
    }
}

/// Table definition in a schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableDef {
    /// Table name
    pub name: String,
    /// Schema name (for PostgreSQL)
    pub schema: Option<String>,
    /// Column definitions
    pub columns: Vec<ColumnDef>,
    /// Primary key column names
    pub primary_key: Vec<String>,
}

impl TableDef {
    /// Create a new table definition.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            schema: None,
            columns: Vec::new(),
            primary_key: Vec::new(),
        }
    }

    /// Set the schema name.
    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    /// Add a column to the table.
    pub fn add_column(&mut self, column: ColumnDef) {
        if column.is_primary_key && !self.primary_key.contains(&column.name) {
            self.primary_key.push(column.name.clone());
        }
        self.columns.push(column);
    }

    /// Get a column by name.
    pub fn get_column(&self, name: &str) -> Option<&ColumnDef> {
        self.columns.iter().find(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// Get a mutable column by name.
    pub fn get_column_mut(&mut self, name: &str) -> Option<&mut ColumnDef> {
        self.columns.iter_mut().find(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// Remove a column by name.
    pub fn remove_column(&mut self, name: &str) -> Option<ColumnDef> {
        if let Some(pos) = self.columns.iter().position(|c| c.name.eq_ignore_ascii_case(name)) {
            self.primary_key.retain(|pk| !pk.eq_ignore_ascii_case(name));
            Some(self.columns.remove(pos))
        } else {
            None
        }
    }

    /// Get the fully qualified table name.
    pub fn full_name(&self) -> String {
        match &self.schema {
            Some(schema) => format!("{}.{}", schema, self.name),
            None => self.name.clone(),
        }
    }
}
