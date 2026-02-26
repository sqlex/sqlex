use sqlex_analyzer::{
    error::AnalyzerError,
    extension::{data_type_ext::DataTypeExt, ident_ext::IdentExt, object_name_ext::ObjectNameExt},
};
use sqlex_common::{dialect::Dialect, types::DataType};
use sqlparser::ast::{ColumnDef, ColumnOption, CreateTable, TableConstraint};

use crate::catalog::{
    Catalog, ParsedTableConstraint, error_code,
    model::{ColumnSchema, ForeignKeyConstraint, KeyConstraint, TableSchema},
};

impl Catalog {
    pub(super) fn apply_create_table(
        &mut self,
        dialect: Dialect,
        create_table: &CreateTable,
    ) -> Result<(), AnalyzerError> {
        if create_table.query.is_some() {
            return Err(AnalyzerError::analysis(
                error_code::CREATE_TABLE_AS_SELECT_UNSUPPORTED,
                "CREATE TABLE AS SELECT is not supported in this iteration",
            ));
        }

        let table_name = create_table.name.to_normalized_string(dialect);
        let original_table_name = create_table.name.to_dotted_string();

        if self.tables.iter().any(|table| table.name == table_name) {
            if create_table.if_not_exists {
                return Ok(());
            }
            return Err(AnalyzerError::analysis(
                error_code::CREATE_TABLE_ALREADY_EXISTS,
                format!("table '{}' already exists", original_table_name),
            ));
        }

        let mut columns = Vec::with_capacity(create_table.columns.len());
        for column_def in &create_table.columns {
            let column = Self::build_column(dialect, column_def);
            if columns
                .iter()
                .any(|existing: &ColumnSchema| existing.name == column.name)
            {
                return Err(AnalyzerError::analysis(
                    error_code::CREATE_TABLE_DUPLICATE_COLUMN,
                    format!(
                        "duplicate column '{}' in table '{}'",
                        column.name, table_name
                    ),
                ));
            }
            columns.push(column);
        }

        let mut table = TableSchema {
            name: table_name.clone(),
            original_name: original_table_name,
            columns,
            primary_key: None,
            unique_keys: Vec::new(),
            foreign_keys: Vec::new(),
        };

        for column_def in &create_table.columns {
            Self::apply_inline_column_constraints(dialect, &mut table, column_def)?;
        }

        for constraint in &create_table.constraints {
            match self.parse_table_constraint(dialect, &table, constraint)? {
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
        }

        Self::enforce_primary_key_nullability(dialect, &mut table);
        self.validate_foreign_keys(&table)?;

        if self
            .tables
            .iter()
            .any(|existing| existing.name == table.name)
        {
            return Err(AnalyzerError::analysis(
                error_code::CATALOG_TABLE_ALREADY_EXISTS,
                format!(
                    "failed to add table: table '{}' already exists",
                    table.original_name
                ),
            ));
        }
        self.tables.push(table);

        Ok(())
    }

    pub(super) fn build_column(dialect: Dialect, column_def: &ColumnDef) -> ColumnSchema {
        let name = column_def.name.to_normalized_string(dialect);
        let nullable = !column_def
            .options
            .iter()
            .any(|option_def| matches!(&option_def.option, ColumnOption::NotNull));

        ColumnSchema {
            name,
            original_name: column_def.name.value.clone(),
            data_type: DataType::from_sql_data_type(dialect, &column_def.data_type),
            nullable,
        }
    }

    pub(super) fn apply_inline_column_constraints(
        dialect: Dialect,
        table: &mut TableSchema,
        column_def: &ColumnDef,
    ) -> Result<(), AnalyzerError> {
        let column_name = column_def.name.to_normalized_string(dialect);
        for option_def in &column_def.options {
            match &option_def.option {
                ColumnOption::Unique { is_primary, .. } => {
                    if *is_primary {
                        let key = table.primary_key.get_or_insert(KeyConstraint {
                            name: option_def.name.as_ref().map(|ident| ident.value.clone()),
                            columns: Vec::new(),
                        });
                        if !key.columns.iter().any(|name| name == &column_name) {
                            key.columns.push(column_name.clone());
                        }
                    } else {
                        table.unique_keys.push(KeyConstraint {
                            name: option_def.name.as_ref().map(|ident| ident.value.clone()),
                            columns: vec![column_name.clone()],
                        });
                    }
                },
                ColumnOption::ForeignKey {
                    foreign_table,
                    referred_columns,
                    ..
                } => {
                    let ref_columns = if referred_columns.is_empty() {
                        vec![column_name.clone()]
                    } else {
                        referred_columns
                            .iter()
                            .map(|ident| ident.to_normalized_string(dialect))
                            .collect()
                    };
                    table.foreign_keys.push(ForeignKeyConstraint {
                        name: option_def.name.as_ref().map(|ident| ident.value.clone()),
                        columns: vec![column_name.clone()],
                        ref_table: foreign_table.to_normalized_string(dialect),
                        ref_columns,
                    });
                },
                _ => {},
            }
        }
        Ok(())
    }

    pub(super) fn parse_table_constraint(
        &self,
        dialect: Dialect,
        table: &TableSchema,
        constraint: &TableConstraint,
    ) -> Result<ParsedTableConstraint, AnalyzerError> {
        match constraint {
            TableConstraint::PrimaryKey { name, columns, .. } => {
                let normalized_columns =
                    Self::normalize_and_validate_columns(dialect, table, columns, "primary key")?;
                Ok(ParsedTableConstraint::PrimaryKey(KeyConstraint {
                    name: name.as_ref().map(|ident| ident.value.clone()),
                    columns: normalized_columns,
                }))
            },
            TableConstraint::Unique { name, columns, .. } => {
                let normalized_columns =
                    Self::normalize_and_validate_columns(dialect, table, columns, "unique key")?;
                Ok(ParsedTableConstraint::UniqueKey(KeyConstraint {
                    name: name.as_ref().map(|ident| ident.value.clone()),
                    columns: normalized_columns,
                }))
            },
            TableConstraint::ForeignKey {
                name,
                columns,
                foreign_table,
                referred_columns,
                ..
            } => {
                let normalized_columns =
                    Self::normalize_and_validate_columns(dialect, table, columns, "foreign key")?;
                let normalized_ref_columns = if referred_columns.is_empty() {
                    normalized_columns.clone()
                } else {
                    referred_columns
                        .iter()
                        .map(|ident| ident.to_normalized_string(dialect))
                        .collect()
                };

                let foreign_key = ForeignKeyConstraint {
                    name: name.as_ref().map(|ident| ident.value.clone()),
                    columns: normalized_columns,
                    ref_table: foreign_table.to_normalized_string(dialect),
                    ref_columns: normalized_ref_columns,
                };

                let mut validation_table = table.clone();
                validation_table.foreign_keys.push(foreign_key.clone());
                self.validate_foreign_keys(&validation_table)?;

                Ok(ParsedTableConstraint::ForeignKey(foreign_key))
            },
            _ => Ok(ParsedTableConstraint::Unsupported),
        }
    }

    pub(super) fn normalize_and_validate_columns(
        dialect: Dialect,
        table: &TableSchema,
        columns: &[sqlparser::ast::Ident],
        label: &str,
    ) -> Result<Vec<String>, AnalyzerError> {
        if columns.is_empty() {
            return Err(AnalyzerError::analysis(
                error_code::CONSTRAINT_COLUMNS_EMPTY,
                format!("{label} must contain at least one column"),
            ));
        }

        let mut normalized_columns = Vec::with_capacity(columns.len());
        for column in columns {
            let normalized = column.to_normalized_string(dialect);
            if !table.has_column(&normalized) {
                return Err(AnalyzerError::analysis(
                    error_code::CONSTRAINT_COLUMN_NOT_FOUND,
                    format!(
                        "column '{}' not found in table '{}'",
                        normalized, table.name
                    ),
                ));
            }
            if normalized_columns.iter().any(|name| name == &normalized) {
                return Err(AnalyzerError::analysis(
                    error_code::CONSTRAINT_DUPLICATE_COLUMN,
                    format!("column '{}' repeated in {}", normalized, label),
                ));
            }
            normalized_columns.push(normalized);
        }
        Ok(normalized_columns)
    }

    pub(super) fn enforce_primary_key_nullability(dialect: Dialect, table: &mut TableSchema) {
        let Some(primary_key) = &table.primary_key else {
            return;
        };

        for column_name in &primary_key.columns {
            let Some(column_index) = table.column_index(column_name) else {
                continue;
            };

            let make_not_null = match dialect {
                Dialect::Postgres | Dialect::MySQL => true,
                Dialect::SQLite => matches!(
                    table.columns[column_index].data_type,
                    DataType::TinyInt
                        | DataType::UnsignedTinyInt
                        | DataType::SmallInt
                        | DataType::UnsignedSmallInt
                        | DataType::Int
                        | DataType::UnsignedInt
                        | DataType::BigInt
                        | DataType::UnsignedBigInt
                ),
            };

            if make_not_null {
                table.columns[column_index].nullable = false;
            }
        }
    }

    pub(super) fn validate_foreign_keys(&self, table: &TableSchema) -> Result<(), AnalyzerError> {
        for foreign_key in &table.foreign_keys {
            if foreign_key.columns.is_empty() {
                return Err(AnalyzerError::analysis(
                    error_code::FOREIGN_KEY_LOCAL_COLUMNS_EMPTY,
                    format!("foreign key in '{}' has no local columns", table.name),
                ));
            }
            if foreign_key.columns.len() != foreign_key.ref_columns.len() {
                return Err(AnalyzerError::analysis(
                    error_code::FOREIGN_KEY_COLUMN_COUNT_MISMATCH,
                    format!(
                        "foreign key in '{}' has mismatched local/referenced column counts",
                        table.name
                    ),
                ));
            }
            for local_column in &foreign_key.columns {
                if !table.has_column(local_column) {
                    return Err(AnalyzerError::analysis(
                        error_code::FOREIGN_KEY_LOCAL_COLUMN_NOT_FOUND,
                        format!(
                            "foreign key references missing local column '{}' in '{}'",
                            local_column, table.name
                        ),
                    ));
                }
            }

            let referenced_table = if foreign_key.ref_table == table.name {
                Some(table)
            } else {
                self.tables
                    .iter()
                    .find(|existing| existing.name == foreign_key.ref_table)
            };

            let Some(referenced_table) = referenced_table else {
                return Err(AnalyzerError::analysis(
                    error_code::FOREIGN_KEY_REF_TABLE_NOT_FOUND,
                    format!(
                        "foreign key references unknown table '{}' from '{}'",
                        foreign_key.ref_table, table.name
                    ),
                ));
            };

            for ref_column in &foreign_key.ref_columns {
                if !referenced_table.has_column(ref_column) {
                    return Err(AnalyzerError::analysis(
                        error_code::FOREIGN_KEY_REF_COLUMN_NOT_FOUND,
                        format!(
                            "foreign key references unknown column '{}.{}'",
                            foreign_key.ref_table, ref_column
                        ),
                    ));
                }
            }
        }

        Ok(())
    }
}
