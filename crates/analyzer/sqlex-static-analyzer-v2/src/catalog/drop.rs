use sqlex_analyzer::{error::AnalyzerError, extension::object_name_ext::ObjectNameExt};
use sqlex_common::dialect::Dialect;
use sqlparser::ast::{ObjectName, ObjectType};

use crate::catalog::{Catalog, error_code};

impl Catalog {
    pub(super) fn apply_drop(
        &mut self,
        dialect: Dialect,
        object_type: &ObjectType,
        if_exists: bool,
        names: &[ObjectName],
    ) -> Result<(), AnalyzerError> {
        if *object_type != ObjectType::Table {
            return Err(AnalyzerError::analysis(
                error_code::DROP_ONLY_TABLE_SUPPORTED,
                "DROP only supports TABLE in this iteration",
            ));
        }

        for object_name in names {
            let normalized_name = object_name.to_normalized_string(dialect);
            if self
                .tables
                .iter()
                .all(|table| table.name != normalized_name)
            {
                if if_exists {
                    continue;
                }
                return Err(AnalyzerError::analysis(
                    error_code::DROP_TABLE_NOT_FOUND,
                    format!("table '{}' does not exist", normalized_name),
                ));
            }

            if self.is_referenced_by_foreign_key(&normalized_name) {
                return Err(AnalyzerError::analysis(
                    error_code::DROP_TABLE_REFERENCED_BY_FOREIGN_KEY,
                    format!(
                        "cannot drop table '{}': referenced by foreign key constraints",
                        normalized_name
                    ),
                ));
            }

            let Some(index) = self
                .tables
                .iter()
                .position(|table| table.name == normalized_name)
            else {
                return Err(AnalyzerError::analysis(
                    error_code::CATALOG_TABLE_NOT_FOUND,
                    format!(
                        "failed to drop table: table '{}' does not exist",
                        normalized_name
                    ),
                ));
            };
            let _ = self.tables.remove(index);
        }

        Ok(())
    }

    pub(super) fn validate_drop_column(
        &self,
        table_index: usize,
        column_name: &str,
    ) -> Result<(), AnalyzerError> {
        let table = &self.tables[table_index];

        if table
            .primary_key
            .as_ref()
            .is_some_and(|key| key.columns.iter().any(|name| name == column_name))
        {
            return Err(AnalyzerError::analysis(
                error_code::DROP_COLUMN_USED_BY_PRIMARY_KEY,
                format!(
                    "cannot drop column '{}.{}': used by primary key",
                    table.name, column_name
                ),
            ));
        }

        if table
            .unique_keys
            .iter()
            .any(|key| key.columns.iter().any(|name| name == column_name))
        {
            return Err(AnalyzerError::analysis(
                error_code::DROP_COLUMN_USED_BY_UNIQUE_KEY,
                format!(
                    "cannot drop column '{}.{}': used by unique key",
                    table.name, column_name
                ),
            ));
        }

        if table
            .foreign_keys
            .iter()
            .any(|key| key.columns.iter().any(|name| name == column_name))
        {
            return Err(AnalyzerError::analysis(
                error_code::DROP_COLUMN_USED_BY_FOREIGN_KEY,
                format!(
                    "cannot drop column '{}.{}': used by foreign key",
                    table.name, column_name
                ),
            ));
        }

        if self.tables.iter().any(|other_table| {
            other_table.foreign_keys.iter().any(|foreign_key| {
                foreign_key.ref_table == table.name
                    && foreign_key
                        .ref_columns
                        .iter()
                        .any(|ref_column| ref_column == column_name)
            })
        }) {
            return Err(AnalyzerError::analysis(
                error_code::DROP_COLUMN_REFERENCED_BY_FOREIGN_KEYS,
                format!(
                    "cannot drop column '{}.{}': referenced by other foreign keys",
                    table.name, column_name
                ),
            ));
        }

        Ok(())
    }

    fn is_referenced_by_foreign_key(&self, table_name: &str) -> bool {
        self.tables.iter().any(|table| {
            table
                .foreign_keys
                .iter()
                .any(|foreign_key| foreign_key.ref_table == table_name)
        })
    }
}
