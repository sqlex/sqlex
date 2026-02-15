//! Static SQL Analyzer
//!
//! A static SQL analyzer that infers result set types and nullability
//! without requiring a database connection.

use sqlex_common::dialect::Dialect;

mod algebra;
mod analyzer;
mod catalog;
mod diagnostics;
mod functions;
mod infer;
mod parser;

/// Static SQL analyzer implementation.
#[derive(Debug)]
pub struct StaticAnalyzer {
    pub(crate) dialect: Dialect,
    pub(crate) catalog: catalog::model::Catalog,
    pub(crate) functions: functions::registry::FunctionRegistry,
}

impl StaticAnalyzer {
    /// Creates a new static analyzer with the given dialect.
    pub fn new(dialect: Dialect) -> Self {
        Self {
            dialect,
            catalog: catalog::model::Catalog::new(),
            functions: functions::registry::FunctionRegistry::new(dialect),
        }
    }
}
