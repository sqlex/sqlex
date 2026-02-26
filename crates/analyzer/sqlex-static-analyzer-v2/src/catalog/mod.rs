#![allow(dead_code)]

use sqlex_analyzer::error::AnalyzerError;
use sqlex_common::dialect::Dialect;
use sqlparser::ast::Statement;

mod alter;
mod create;
mod drop;
pub(crate) mod error_code;
pub(crate) mod model;

#[derive(Debug, Clone, Default)]
pub(crate) struct Catalog {
    pub(crate) tables: Vec<model::TableSchema>,
}

enum ParsedTableConstraint {
    PrimaryKey(model::KeyConstraint),
    UniqueKey(model::KeyConstraint),
    ForeignKey(model::ForeignKeyConstraint),
    Unsupported,
}

impl Catalog {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn execute(
        &mut self,
        dialect: Dialect,
        statement: &Statement,
    ) -> Result<(), AnalyzerError> {
        match statement {
            Statement::CreateTable(create_table) => self.apply_create_table(dialect, create_table),
            Statement::AlterTable {
                name,
                if_exists,
                operations,
                ..
            } => self.apply_alter_table(dialect, name, *if_exists, operations),
            Statement::Drop {
                object_type,
                if_exists,
                names,
                ..
            } => self.apply_drop(dialect, object_type, *if_exists, names),
            other => Err(AnalyzerError::analysis(
                error_code::DISPATCH_UNSUPPORTED_STATEMENT,
                format!("unsupported statement in execute: {}", other),
            )),
        }
    }

    pub(crate) fn get_table(&self, table_name: &str) -> Result<&model::TableSchema, AnalyzerError> {
        self.tables
            .iter()
            .find(|table| table.name == table_name)
            .ok_or_else(|| {
                AnalyzerError::analysis(
                    error_code::CATALOG_TABLE_NOT_FOUND,
                    format!("table '{}' does not exist", table_name),
                )
            })
    }

    pub(crate) fn get_all_tables(&self) -> Vec<model::TableSchema> {
        self.tables.clone()
    }
}
