//! Core types for sqlex database schema analyzer.
//!
//! This crate defines the fundamental types used across all sqlex crates:
//! - SQL dialects (PostgreSQL, MySQL, SQLite)
//! - SQL data types
//! - Table and column definitions
//! - Result column metadata

mod dialect;
mod sql_type;
mod table;
mod result;

pub use dialect::Dialect;
pub use sql_type::SqlType;
pub use table::{ColumnDef, TableDef};
pub use result::ResultColumn;
