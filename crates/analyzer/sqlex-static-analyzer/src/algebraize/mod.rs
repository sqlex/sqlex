use std::collections::HashMap;

use sqlex_common::dialect::Dialect;
use sqlparser::{
    ast::Statement,
    dialect::{Dialect as SqlParserDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect},
    parser::Parser,
};

use crate::{catalog::Catalog, diagnostics::Diagnostic, ir::relational::RelationalExpr};

mod cte;
mod expr;
mod from;
mod names;
mod select;
mod set_ops;
mod validate;

pub struct AlgebraizeResult {
    pub expr: Option<RelationalExpr>,
    pub diagnostics: Vec<Diagnostic>,
}

pub(crate) struct Algebraizer<'a> {
    pub(super) dialect: Dialect,
    pub(super) catalog: &'a Catalog,
    pub(super) diagnostics: Vec<Diagnostic>,
    /// CTE definitions available in current scope
    pub(super) cte_scope: HashMap<String, CteEntry>,
}

#[derive(Debug, Clone)]
pub(super) struct CteEntry {
    pub(super) expr: RelationalExpr,
    #[allow(dead_code)]
    pub(super) column_names: Vec<String>,
}

impl<'a> Algebraizer<'a> {
    pub(crate) fn new(dialect: Dialect, catalog: &'a Catalog) -> Self {
        Self {
            dialect,
            catalog,
            diagnostics: Vec::new(),
            cte_scope: HashMap::new(),
        }
    }

    pub(crate) fn algebraize(mut self, sql: &str) -> AlgebraizeResult {
        let expr = self.algebraize_sql(sql);
        AlgebraizeResult {
            expr,
            diagnostics: self.diagnostics,
        }
    }

    fn algebraize_sql(&mut self, sql: &str) -> Option<RelationalExpr> {
        let dialect: Box<dyn SqlParserDialect> = match self.dialect {
            Dialect::Postgres => Box::new(PostgreSqlDialect {}),
            Dialect::MySQL => Box::new(MySqlDialect {}),
            Dialect::SQLite => Box::new(SQLiteDialect {}),
        };
        let statements = match Parser::parse_sql(dialect.as_ref(), sql) {
            Ok(stmts) => stmts,
            Err(e) => {
                self.diagnostics
                    .push(Diagnostic::parse_error(format!("Parse error: {e}")));
                return None;
            },
        };

        if statements.len() != 1 {
            self.diagnostics.push(Diagnostic::invalid_statement(
                "Expected exactly one statement",
            ));
            return None;
        }

        match &statements[0] {
            Statement::Query(query) => self.algebraize_query(query),
            _ => {
                self.diagnostics
                    .push(Diagnostic::invalid_statement("Expected a SELECT query"));
                None
            },
        }
    }
}
