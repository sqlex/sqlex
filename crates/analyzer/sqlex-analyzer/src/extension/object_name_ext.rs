use sqlex_common::dialect::Dialect;
use sqlparser::ast::ObjectName;

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
            .map(|ident| match dialect {
                Dialect::Postgres => {
                    // PostgreSQL: unquoted identifiers are case-insensitive (converted to lowercase)
                    if ident.quote_style.is_none() {
                        ident.value.to_lowercase()
                    } else {
                        ident.value.clone()
                    }
                },
                Dialect::MySQL | Dialect::SQLite => {
                    // MySQL and SQLite: use identifier as-is
                    ident.value.clone()
                },
            })
            .collect::<Vec<_>>()
            .join(".")
    }
}
