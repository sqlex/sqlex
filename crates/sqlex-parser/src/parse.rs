//! SQL parsing functions.

use sqlex_types::Dialect;
use sqlparser::{
    ast::Statement,
    dialect::{Dialect as SqlParserDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect},
    parser::Parser,
};

use crate::ParseError;

/// Get the sqlparser dialect for our dialect enum.
fn get_dialect(dialect: Dialect) -> Box<dyn SqlParserDialect> {
    match dialect {
        Dialect::PostgreSQL => Box::new(PostgreSqlDialect {}),
        Dialect::MySQL => Box::new(MySqlDialect {}),
        Dialect::SQLite => Box::new(SQLiteDialect {}),
    }
}

/// Parse SQL string into a list of statements.
///
/// # Arguments
/// * `sql` - The SQL string to parse
/// * `dialect` - The SQL dialect to use
///
/// # Returns
/// A vector of parsed statements
pub fn parse(sql: &str, dialect: Dialect) -> Result<Vec<Statement>, ParseError> {
    let sql = sql.trim();
    if sql.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    let dialect = get_dialect(dialect);
    let statements = Parser::parse_sql(dialect.as_ref(), sql)?;
    Ok(statements)
}

/// Parse a single SQL statement.
///
/// # Arguments
/// * `sql` - The SQL string to parse (should contain exactly one statement)
/// * `dialect` - The SQL dialect to use
///
/// # Returns
/// The parsed statement
pub fn parse_one(sql: &str, dialect: Dialect) -> Result<Statement, ParseError> {
    let statements = parse(sql, dialect)?;
    match statements.len() {
        0 => Err(ParseError::EmptyInput),
        1 => Ok(statements.into_iter().next().unwrap()),
        n => Err(ParseError::MultipleStatements(n)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_create_table_postgresql() {
        let sql = "CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR(100) NOT NULL)";
        let stmts = parse(sql, Dialect::PostgreSQL).unwrap();
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::CreateTable(_)));
    }

    #[test]
    fn test_parse_create_table_mysql() {
        let sql =
            "CREATE TABLE users (id INT AUTO_INCREMENT PRIMARY KEY, name VARCHAR(100) NOT NULL)";
        let stmts = parse(sql, Dialect::MySQL).unwrap();
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::CreateTable(_)));
    }

    #[test]
    fn test_parse_select() {
        let sql = "SELECT id, name FROM users WHERE id = 1";
        let stmt = parse_one(sql, Dialect::PostgreSQL).unwrap();
        assert!(matches!(stmt, Statement::Query(_)));
    }

    #[test]
    fn test_parse_empty() {
        let result = parse("", Dialect::PostgreSQL);
        assert!(matches!(result, Err(ParseError::EmptyInput)));
    }

    #[test]
    fn test_parse_multiple_statements() {
        let sql = "SELECT 1; SELECT 2";
        let result = parse_one(sql, Dialect::PostgreSQL);
        assert!(matches!(result, Err(ParseError::MultipleStatements(2))));
    }
}
