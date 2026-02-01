//! Migration struct definition.

/// A single migration file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    /// Version number extracted from filename
    pub version: u64,
    /// Description from filename
    pub name: String,
    /// SQL content
    pub sql: String,
    /// Original filename
    pub filename: String,
}

impl Migration {
    /// Create a new migration.
    pub fn new(version: u64, name: impl Into<String>, sql: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            version,
            filename: format!("V{}__{}.sql", version, name),
            name,
            sql: sql.into(),
        }
    }
}

impl PartialOrd for Migration {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Migration {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.version.cmp(&other.version)
    }
}
