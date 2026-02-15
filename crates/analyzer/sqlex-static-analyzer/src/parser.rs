use sqlex_common::dialect::Dialect;
use sqlparser::{
    ast::Statement,
    dialect::{MySqlDialect, PostgreSqlDialect, SQLiteDialect},
    parser::Parser,
};

use crate::diagnostics::{Diagnostic, Phase};

pub(crate) fn parse_statements(dialect: Dialect, sql: &str) -> Result<Vec<Statement>, Diagnostic> {
    let statements = match dialect {
        Dialect::Postgres => Parser::parse_sql(&PostgreSqlDialect {}, sql),
        Dialect::MySQL => Parser::parse_sql(&MySqlDialect {}, sql),
        Dialect::SQLite => Parser::parse_sql(&SQLiteDialect {}, sql),
    }
    .map_err(|err| Diagnostic::new("P1001", Phase::Parse, format!("failed to parse SQL: {err}")))?;

    if statements.is_empty() {
        return Err(Diagnostic::new(
            "P1002",
            Phase::Parse,
            "empty SQL is not allowed",
        ));
    }

    Ok(statements)
}

pub(crate) fn parse_query_statement(dialect: Dialect, sql: &str) -> Result<Statement, Diagnostic> {
    let mut statements = parse_statements(dialect, sql)?;

    if statements.len() != 1 {
        return Err(Diagnostic::new(
            "P1003",
            Phase::Parse,
            "analyze expects exactly one SQL statement",
        ));
    }

    let statement = statements.remove(0);
    if matches!(statement, Statement::Query(_)) {
        Ok(statement)
    } else {
        Err(Diagnostic::new(
            "P1004",
            Phase::Parse,
            "analyze only supports SELECT/WITH query statements",
        ))
    }
}
