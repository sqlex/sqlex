//! Core types for sqlex database schema analyzer.
//!
//! This crate defines the fundamental types used across all sqlex crates:
//! - SQL dialects (PostgreSQL, MySQL, SQLite)
//! - SQL data types
//! - Table and column definitions
//! - Result column metadata

mod dialect;
mod result;
mod sql_type;
mod table;

pub use dialect::Dialect;
pub use result::ResultColumn;
pub use sql_type::SqlType;
pub use table::{ColumnDef, TableDef};
