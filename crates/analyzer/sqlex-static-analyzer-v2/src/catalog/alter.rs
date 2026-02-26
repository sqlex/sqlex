use sqlex_analyzer::{
    error::AnalyzerError,
    extension::{ident_ext::IdentExt, object_name_ext::ObjectNameExt},
};
use sqlex_common::dialect::Dialect;
use sqlparser::ast::{AlterTableOperation, ObjectName, TableConstraint};

use crate::catalog::{Catalog, ParsedTableConstraint, error_code};

impl Catalog {
    pub(super) fn apply_alter_table(
        &mut self,
        dialect: Dialect,
        table_name_ast: &ObjectName,
        if_exists: bool,
        operations: &[AlterTableOperation],
    ) -> Result<(), AnalyzerError> {
        let table_name = table_name_ast.to_normalized_string(dialect);
        let Some(table_index) = self
            .tables
            .iter()
            .position(|table| table.name == table_name)
        else {
            if if_exists {
                return Ok(());
            }
            return Err(AnalyzerError::analysis(
                error_code::ALTER_TABLE_TABLE_NOT_FOUND,
                format!("table '{}' not found for ALTER TABLE", table_name),
            ));
        };

        for operation in operations {
            match operation {
                AlterTableOperation::AddColumn {
                    if_not_exists,
                    column_def,
                    ..
                } => {
                    let column = Self::build_column(dialect, column_def);
                    let table = &mut self.tables[table_index];
                    if table.has_column(&column.name) {
                        if *if_not_exists {
                            continue;
                        }
                        return Err(AnalyzerError::analysis(
                            error_code::ALTER_TABLE_ADD_COLUMN_EXISTS,
                            format!(
                                "column '{}' already exists in '{}'",
                                column.name, table.name
                            ),
                        ));
                    }
                    table.columns.push(column);
                    Self::apply_inline_column_constraints(dialect, table, column_def)?;
                    Self::enforce_primary_key_nullability(dialect, table);
                },
                AlterTableOperation::DropColumn {
                    column_name,
                    if_exists,
                    ..
                } => {
                    let normalized_column = column_name.to_normalized_string(dialect);
                    self.validate_drop_column(table_index, &normalized_column)?;
                    let table = &mut self.tables[table_index];
                    let Some(column_index) = table.column_index(&normalized_column) else {
                        if *if_exists {
                            continue;
                        }
                        return Err(AnalyzerError::analysis(
                            error_code::ALTER_TABLE_DROP_COLUMN_NOT_FOUND,
                            format!(
                                "column '{}' not found in table '{}'",
                                normalized_column, table.name
                            ),
                        ));
                    };
                    table.columns.remove(column_index);
                },
                AlterTableOperation::AddConstraint(constraint) => {
                    if matches!(dialect, Dialect::SQLite) {
                        return Err(AnalyzerError::analysis(
                            error_code::ALTER_TABLE_ADD_CONSTRAINT_UNSUPPORTED_SQLITE,
                            "SQLite does not support ALTER TABLE ADD CONSTRAINT",
                        ));
                    }
                    self.apply_constraint_by_index(dialect, table_index, constraint)?;
                },
                _ => {
                    return Err(AnalyzerError::analysis(
                        error_code::ALTER_TABLE_OPERATION_UNSUPPORTED,
                        "unsupported ALTER TABLE operation in this iteration",
                    ));
                },
            }
        }

        Ok(())
    }

    fn apply_constraint_by_index(
        &mut self,
        dialect: Dialect,
        table_index: usize,
        constraint: &TableConstraint,
    ) -> Result<(), AnalyzerError> {
        let table_snapshot = self.tables[table_index].clone();
        let parsed_constraint =
            self.parse_table_constraint(dialect, &table_snapshot, constraint)?;

        {
            let table = &mut self.tables[table_index];
            match parsed_constraint {
                ParsedTableConstraint::PrimaryKey(key) => {
                    table.primary_key = Some(key);
                },
                ParsedTableConstraint::UniqueKey(key) => {
                    table.unique_keys.push(key);
                },
                ParsedTableConstraint::ForeignKey(foreign_key) => {
                    table.foreign_keys.push(foreign_key);
                },
                ParsedTableConstraint::Unsupported => {},
            }
            Self::enforce_primary_key_nullability(dialect, table);
        }

        let table_after_mutation = self.tables[table_index].clone();
        self.validate_foreign_keys(&table_after_mutation)?;
        Ok(())
    }
}
