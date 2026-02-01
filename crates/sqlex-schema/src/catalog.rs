//! Catalog trait for schema abstraction.

use sqlex_types::TableDef;

/// A catalog providing access to table definitions.
pub trait Catalog {
    /// Get a table definition by name.
    fn get_table(&self, name: &str) -> Option<&TableDef>;

    /// Check if a table exists.
    fn has_table(&self, name: &str) -> bool;
}
