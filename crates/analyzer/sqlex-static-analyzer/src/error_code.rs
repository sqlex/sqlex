//! Parse error code definitions.
//!
//! Numbering policy:
//! - Format: `<module><major><minor>`, where module is `P` for parse.
//! - `<major>`: 2 digits (00-99), top-level category.
//! - `<minor>`: 2 digits (00-99), sub-category.
//! - Parse majors:
//!   - `00`: parser entry and statement shape checks

/// SQL parser failed to parse the input SQL.
pub(crate) const PARSE_SQL_FAILED: &str = "P0000";
/// Empty SQL is not allowed.
pub(crate) const PARSE_EMPTY_SQL: &str = "P0001";
/// Exactly one SQL statement is required.
pub(crate) const PARSE_EXPECT_SINGLE_STATEMENT: &str = "P0002";
