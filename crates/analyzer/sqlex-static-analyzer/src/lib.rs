//! Static SQL Analyzer
//!
//! A static SQL analyzer that infers result set types and nullability
//! without requiring a database connection.

use sqlex_common::dialect::Dialect;

mod algebraizer;
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
    pub(crate) catalog: catalog::Catalog,
    pub(crate) functions: functions::FunctionRegistry,
}

impl StaticAnalyzer {
    /// Creates a new static analyzer with the given dialect.
    pub fn new(dialect: Dialect) -> Self {
        Self {
            dialect,
            catalog: catalog::Catalog::new(),
            functions: functions::FunctionRegistry::new(dialect),
        }
    }
}
