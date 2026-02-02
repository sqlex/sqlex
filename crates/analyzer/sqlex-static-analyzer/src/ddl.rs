//! DDL parser for building and maintaining schema
//!
//! Parses CREATE TABLE, ALTER TABLE, and DROP TABLE statements
//! to build and maintain the schema.

use sqlex_analyzer::AnalyzerError;
use sqlparser::{
    dialect::{Dialect as SqlParserDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect},
    parser::Parser,
};

use crate::schema::{ColumnDef, Dialect, ForeignKeyDef, Schema, TableDef};

type Result<T> = std::result::Result<T, AnalyzerError>;

impl Schema {
    /// Parse and execute DDL statement(s)
    pub fn execute_ddl(&mut self, sql: &str) -> Result<()> {
        let dialect = self.get_sqlparser_dialect();
        let statements = Parser::parse_sql(dialect.as_ref(), sql)
            .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;

        for stmt in statements {
            self.execute_statement(&stmt)?;
        }

        self.rebuild_fk_index();
        Ok(())
    }

    /// Execute a single parsed statement
    fn execute_statement(&mut self, stmt: &sqlparser::ast::Statement) -> Result<()> {
        use sqlparser::ast::Statement;

        match stmt {
            Statement::CreateTable(_) => self.handle_create_table(stmt),
            Statement::AlterTable { .. } => self.handle_alter_table(stmt),
            Statement::Drop { .. } => self.handle_drop(stmt),
            _ => Ok(()), // Ignore other statements
        }
    }

    /// Handle CREATE TABLE statement
    fn handle_create_table(&mut self, _stmt: &sqlparser::ast::Statement) -> Result<()> {
        todo!("parse CREATE TABLE and add to schema")
    }

    /// Handle ALTER TABLE statement
    fn handle_alter_table(&mut self, _stmt: &sqlparser::ast::Statement) -> Result<()> {
        todo!("handle ALTER TABLE operations: AddColumn, DropColumn, AddConstraint, etc.")
    }

    /// Handle DROP statement
    fn handle_drop(&mut self, _stmt: &sqlparser::ast::Statement) -> Result<()> {
        todo!("handle DROP TABLE")
    }

    /// Get the sqlparser dialect for the current schema dialect
    fn get_sqlparser_dialect(&self) -> Box<dyn SqlParserDialect> {
        match self.dialect {
            Dialect::PostgreSQL => Box::new(PostgreSqlDialect {}),
            Dialect::MySQL => Box::new(MySqlDialect {}),
            Dialect::SQLite => Box::new(SQLiteDialect {}),
        }
    }
}

/// Parse a column definition from sqlparser AST
pub fn parse_column_def(_col: &sqlparser::ast::ColumnDef) -> Result<ColumnDef> {
    todo!("parse column name, type, and constraints")
}

/// Parse a table constraint from sqlparser AST
pub fn parse_table_constraint(
    _constraint: &sqlparser::ast::TableConstraint,
    _table: &mut TableDef,
) -> Result<()> {
    todo!("parse PRIMARY KEY, UNIQUE, FOREIGN KEY constraints")
}

/// Parse a foreign key constraint
pub fn parse_foreign_key(_constraint: &sqlparser::ast::TableConstraint) -> Result<ForeignKeyDef> {
    todo!("parse FOREIGN KEY constraint")
}

/// Map sqlparser DataType to our DataType
pub fn map_data_type(_sql_type: &sqlparser::ast::DataType) -> sqlex_common::DataType {
    todo!("map sqlparser types to our DataType enum")
}
