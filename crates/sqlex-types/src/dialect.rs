//! Database dialect definitions.

/// Supported SQL database dialects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Dialect {
    /// PostgreSQL dialect
    #[default]
    PostgreSQL,
    /// MySQL dialect
    MySQL,
    /// SQLite dialect
    SQLite,
}

impl Dialect {
    /// Returns the dialect name as a string.
    pub fn name(&self) -> &'static str {
        match self {
            Dialect::PostgreSQL => "PostgreSQL",
            Dialect::MySQL => "MySQL",
            Dialect::SQLite => "SQLite",
        }
    }
}

impl std::fmt::Display for Dialect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
