//! Configuration types for test fixtures.

use std::path::Path;

use serde::Deserialize;

use crate::{Error, Result};

/// Supported SQL dialects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Dialect {
    Postgresql,
    Mysql,
    Sqlite,
}

impl std::fmt::Display for Dialect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dialect::Postgresql => write!(f, "postgresql"),
            Dialect::Mysql => write!(f, "mysql"),
            Dialect::Sqlite => write!(f, "sqlite"),
        }
    }
}

/// A test suite loaded from a TOML file.
#[derive(Debug, Clone, Deserialize)]
pub struct TestSuite {
    /// Schema configuration shared by all tests in this suite.
    pub schema: SchemaConfig,
    /// Individual test cases.
    pub tests: Vec<TestCase>,
}

/// Schema configuration for a test suite.
#[derive(Debug, Clone, Deserialize)]
pub struct SchemaConfig {
    /// The SQL dialect for this test suite.
    pub dialect: Dialect,
    /// Migration SQL to set up the schema.
    pub migration: String,
}

/// A single test case within a test suite.
#[derive(Debug, Clone, Deserialize)]
pub struct TestCase {
    /// Name of the test case.
    pub name: String,
    /// SQL query to analyze.
    pub query: String,
    /// Optional description of what this test validates.
    #[serde(default)]
    pub description: Option<String>,
    /// Whether this test is expected to fail (for negative testing).
    #[serde(default)]
    pub expect_error: bool,
}

impl TestSuite {
    /// Load a test suite from a TOML file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        Self::from_str(&content)
    }

    /// Parse a test suite from a TOML string.
    pub fn from_str(content: &str) -> Result<Self> {
        toml::from_str(content).map_err(Error::from)
    }

    /// Get the dialect for this test suite.
    pub fn dialect(&self) -> Dialect {
        self.schema.dialect
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_test_suite() {
        let toml = r#"
[schema]
dialect = "postgresql"
migration = """
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL
);
"""

[[tests]]
name = "simple_select"
query = "SELECT id, name FROM users"

[[tests]]
name = "select_with_param"
query = "SELECT * FROM users WHERE id = $1"
description = "Test parameter binding"
"#;

        let suite = TestSuite::from_str(toml).unwrap();
        assert_eq!(suite.schema.dialect, Dialect::Postgresql);
        assert_eq!(suite.tests.len(), 2);
        assert_eq!(suite.tests[0].name, "simple_select");
        assert_eq!(suite.tests[1].name, "select_with_param");
    }
}
