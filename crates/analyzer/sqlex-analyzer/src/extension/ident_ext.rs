use sqlex_common::dialect::Dialect;
use sqlparser::ast::Ident;

/// Extension trait for sqlparser Ident.
pub trait IdentExt {
    /// Convert identifier to normalized string according to dialect rules.
    /// For PostgreSQL: unquoted identifiers are converted to lowercase.
    /// For MySQL/SQLite: identifiers are used as-is.
    fn to_normalized_string(&self, dialect: Dialect) -> String;
}

impl IdentExt for Ident {
    fn to_normalized_string(&self, dialect: Dialect) -> String {
        match dialect {
            Dialect::Postgres => {
                if self.quote_style.is_none() {
                    self.value.to_lowercase()
                } else {
                    self.value.clone()
                }
            },
            Dialect::MySQL | Dialect::SQLite => self.value.clone(),
        }
    }
}
