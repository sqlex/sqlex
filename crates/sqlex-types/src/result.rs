//! Result column metadata for query analysis.

use crate::SqlType;

/// Metadata for a column in a query result set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultColumn {
    /// Column name or alias
    pub name: String,
    /// Inferred data type
    pub data_type: SqlType,
    /// Whether the column can be NULL
    pub nullable: bool,
    /// Source table name (if from a table)
    pub source_table: Option<String>,
    /// Source column name (if from a table column)
    pub source_column: Option<String>,
}

impl ResultColumn {
    /// Create a new result column.
    pub fn new(name: impl Into<String>, data_type: SqlType, nullable: bool) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable,
            source_table: None,
            source_column: None,
        }
    }

    /// Create a result column from a table column.
    pub fn from_table_column(
        name: impl Into<String>,
        data_type: SqlType,
        nullable: bool,
        table: impl Into<String>,
        column: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable,
            source_table: Some(table.into()),
            source_column: Some(column.into()),
        }
    }

    /// Set the source table.
    pub fn with_source_table(mut self, table: impl Into<String>) -> Self {
        self.source_table = Some(table.into());
        self
    }

    /// Set the source column.
    pub fn with_source_column(mut self, column: impl Into<String>) -> Self {
        self.source_column = Some(column.into());
        self
    }
}
