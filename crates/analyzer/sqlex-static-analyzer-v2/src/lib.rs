//! Static SQL Analyzer V2 core.

use async_trait::async_trait;
use sqlex_analyzer::{Analyzer, Result};
use sqlex_common::{
    dialect::Dialect,
    types::{ColumnInfo, ResultSet, Table},
};
use sqlparser::{
    ast::Statement,
    dialect::{MySqlDialect, PostgreSqlDialect, SQLiteDialect},
    parser::Parser,
};

pub mod arena;
mod binder;
mod builder;
pub mod cardinality;
pub(crate) mod catalog;
#[allow(dead_code)]
pub(crate) mod functions;
pub mod node;
pub mod placeholder;

/// Static SQL analyzer implementation.
#[derive(Debug)]
pub struct StaticAnalyzer {
    pub(crate) dialect: Dialect,
    pub(crate) catalog: catalog::Catalog,
    #[allow(dead_code)]
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

    fn parse_statement(&self, sql: &str) -> Result<Statement> {
        let statements = match self.dialect {
            Dialect::Postgres => Parser::parse_sql(&PostgreSqlDialect {}, sql),
            Dialect::MySQL => Parser::parse_sql(&MySqlDialect {}, sql),
            Dialect::SQLite => Parser::parse_sql(&SQLiteDialect {}, sql),
        }
        .map_err(|err| {
            sqlex_analyzer::error::AnalyzerError::analysis(
                "P0000",
                format!("failed to parse SQL: {err}"),
            )
        })?;

        if statements.is_empty() {
            return Err(sqlex_analyzer::error::AnalyzerError::analysis(
                "P0001",
                "empty SQL is not allowed",
            ));
        }

        if statements.len() > 1 {
            return Err(sqlex_analyzer::error::AnalyzerError::analysis(
                "P0002",
                "exactly one SQL statement is required",
            ));
        }

        statements.into_iter().next().ok_or_else(|| {
            sqlex_analyzer::error::AnalyzerError::analysis("P0001", "empty SQL is not allowed")
        })
    }
}

#[async_trait]
impl Analyzer for StaticAnalyzer {
    async fn execute(&mut self, sql: &str) -> Result<()> {
        let statement = self.parse_statement(sql)?;
        self.catalog.execute(self.dialect, &statement)?;

        Ok(())
    }

    async fn analyze(&self, sql: &str) -> Result<ResultSet> {
        let statement = self.parse_statement(sql)?;

        let tree = builder::RelationBuilder::new(self.dialect, &self.catalog, &self.functions)
            .build(statement)?;
        let _bound_tree = binder::RelationBinder::new(self.dialect, &self.catalog, tree).bind()?;

        Err(sqlex_analyzer::error::AnalyzerError::todo(
            "result-set inference is not implemented for pure relation tree yet",
        ))
    }

    async fn get_all_tables(&self) -> Result<Vec<Table>> {
        Ok(self
            .catalog
            .get_all_tables()
            .into_iter()
            .map(|table| Table {
                name: table.name,
                columns: table
                    .columns
                    .into_iter()
                    .map(|column| ColumnInfo {
                        name: column.name,
                        data_type: column.data_type,
                        nullability: column.nullable,
                    })
                    .collect(),
            })
            .collect())
    }
}
