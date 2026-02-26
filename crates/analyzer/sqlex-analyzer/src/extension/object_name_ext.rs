use sqlex_common::dialect::Dialect;
use sqlparser::ast::ObjectName;

use crate::extension::ident_ext::IdentExt;

/// Extension trait for sqlparser ObjectName
pub trait ObjectNameExt {
    /// Convert ObjectName to a dotted string (e.g. "schema.table")
    fn to_dotted_string(&self) -> String;

    /// Convert ObjectName to a normalized dotted string according to dialect rules
    /// For PostgreSQL: unquoted identifiers are converted to lowercase
    /// For MySQL/SQLite: identifiers are used as-is
    fn to_normalized_string(&self, dialect: Dialect) -> String;
}

impl ObjectNameExt for ObjectName {
    fn to_dotted_string(&self) -> String {
        self.0
            .iter()
            .map(|ident| ident.value.clone())
            .collect::<Vec<_>>()
            .join(".")
    }

    fn to_normalized_string(&self, dialect: Dialect) -> String {
        self.0
            .iter()
            .map(|ident| ident.to_normalized_string(dialect))
            .collect::<Vec<_>>()
            .join(".")
    }
}
