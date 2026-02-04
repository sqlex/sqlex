//! Catalog Component
//!
//! Manages database catalog information and handles all DDL statements.

use std::collections::HashMap;

use sqlex_common::dialect::Dialect;

use crate::catalog::types::{ForeignKeyDef, TableDef};

pub mod ddl;
pub mod types;

/// Database catalog containing all table definitions
#[derive(Debug)]
pub struct Catalog {
    pub(super) dialect: Dialect,
    pub tables: HashMap<String, TableDef>,
    /// Reverse index: target_table -> [(source_table, fk)]
    /// Used for quickly querying "who references this table"
    #[allow(dead_code)] // Will be used when FK lookup is implemented
    fk_reverse_index: HashMap<String, Vec<(String, ForeignKeyDef)>>,
}

impl Catalog {
    /// Create a new empty catalog with the given dialect
    pub fn new(dialect: Dialect) -> Self {
        Self {
            dialect,
            tables: HashMap::new(),
            fk_reverse_index: HashMap::new(),
        }
    }

    /// Rebuild the foreign key reverse index
    pub fn rebuild_fk_index(&mut self) {
        self.fk_reverse_index.clear();
        for (table_name, table) in &self.tables {
            for fk in &table.foreign_keys {
                self.fk_reverse_index
                    .entry(fk.ref_table.clone())
                    .or_default()
                    .push((table_name.clone(), fk.clone()));
            }
        }
    }

    /// Get all foreign keys that reference a given table
    pub fn get_references_to(&self, table: &str) -> Vec<&ForeignKeyDef> {
        self.fk_reverse_index
            .get(table)
            .map(|fks| fks.iter().map(|(_, fk)| fk).collect())
            .unwrap_or_default()
    }

    /// Get a table definition by name
    pub fn get_table(&self, name: &str) -> Option<&TableDef> {
        self.tables.get(name)
    }

    /// Add a table to the catalog
    pub fn add_table(&mut self, table: TableDef) {
        self.tables.insert(table.name.clone(), table);
    }
}
