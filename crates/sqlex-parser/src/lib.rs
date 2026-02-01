//! SQL parsing wrapper for sqlex.
//!
//! This crate provides a unified interface for parsing SQL statements
//! across different database dialects (PostgreSQL, MySQL, SQLite).

mod error;
mod parse;
mod type_convert;

pub use error::ParseError;
pub use parse::{parse, parse_one};
pub use type_convert::convert_data_type;

// Re-export sqlparser types that are commonly used
pub use sqlparser::ast::{
    AlterColumnOperation, AlterTableOperation, ColumnDef as SqlColumnDef, ColumnOption,
    ColumnOptionDef, CreateTable, DataType as SqlDataType, Expr, Ident, ObjectName, ObjectType,
    Query, Select, SelectItem, SetExpr, Statement, TableConstraint, TableFactor, TableWithJoins,
};

// Re-export sqlparser module for advanced use
pub use sqlparser;
