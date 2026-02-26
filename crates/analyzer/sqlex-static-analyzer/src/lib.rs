//! Static SQL Analyzer
//!
//! A static SQL analyzer that infers result set types and nullability
//! without requiring a database connection.

use sqlex_common::dialect::Dialect;
use sqlparser::{
    ast::Statement,
    dialect::{MySqlDialect, PostgreSqlDialect, SQLiteDialect},
    parser::Parser,
};

mod algebraizer;
mod analyzer;
mod catalog;
mod diagnostics;
mod functions;
mod infer;

use crate::diagnostics::{Diagnostic, Phase};

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

    pub(crate) fn parse_statement(&self, sql: &str) -> Result<Statement, Diagnostic> {
        let mut statements = match self.dialect {
            Dialect::Postgres => Parser::parse_sql(&PostgreSqlDialect {}, sql),
            Dialect::MySQL => Parser::parse_sql(&MySqlDialect {}, sql),
            Dialect::SQLite => Parser::parse_sql(&SQLiteDialect {}, sql),
        }
        .map_err(|err| {
            Diagnostic::new("P1001", Phase::Parse, format!("failed to parse SQL: {err}"))
        })?;

        if statements.is_empty() {
            return Err(Diagnostic::new(
                "P1002",
                Phase::Parse,
                "empty SQL is not allowed",
            ));
        }

        if statements.len() != 1 {
            return Err(Diagnostic::new(
                "P1003",
                Phase::Parse,
                "exactly one SQL statement is required",
            ));
        }

        Ok(statements.remove(0))
    }
}
