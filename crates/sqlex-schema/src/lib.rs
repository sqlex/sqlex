//! Schema registry for sqlex.
//!
//! This crate provides the `SchemaRegistry` which maintains a collection of
//! table definitions and can be updated by applying DDL statements.

mod error;
mod registry;

pub use error::SchemaError;
pub use registry::SchemaRegistry;
