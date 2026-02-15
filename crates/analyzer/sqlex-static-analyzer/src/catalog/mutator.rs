use sqlex_common::{dialect::Dialect, types::DataType};
use sqlparser::ast::{
    AlterTableOperation, ColumnDef, ColumnOption, CreateTable, ObjectType, Statement,
    TableConstraint,
};

use crate::{
    catalog::{
        ddl_type_map,
        model::{Catalog, ColumnSchema, ForeignKeyConstraint, KeyConstraint, TableSchema},
        normalize::{normalize_ident, normalize_object_name, original_object_name},
    },
    diagnostics::{Diagnostic, Phase},
};

pub(crate) struct CatalogMutator {
    dialect: Dialect,
}

enum ParsedTableConstraint {
    PrimaryKey(KeyConstraint),
    UniqueKey(KeyConstraint),
    ForeignKey(ForeignKeyConstraint),
    Unsupported,
}

impl CatalogMutator {
    pub(crate) fn new(dialect: Dialect) -> Self {
        Self { dialect }
    }

    pub(crate) fn apply_statement(
        &self,
        catalog: &mut Catalog,
        statement: &Statement,
    ) -> Result<(), Diagnostic> {
        match statement {
            Statement::CreateTable(create_table) => self.apply_create_table(catalog, create_table),
            Statement::AlterTable {
                name,
                if_exists,
                operations,
                ..
            } => self.apply_alter_table(catalog, name, *if_exists, operations),
            Statement::Drop {
                object_type,
                if_exists,
                names,
                ..
            } => self.apply_drop(catalog, object_type, *if_exists, names),
            other => Err(Diagnostic::new(
                "C2001",
                Phase::Catalog,
                format!("unsupported statement in execute: {}", other),
            )),
        }
    }

    fn apply_create_table(
        &self,
        catalog: &mut Catalog,
        create_table: &CreateTable,
    ) -> Result<(), Diagnostic> {
        if create_table.query.is_some() {
            return Err(Diagnostic::new(
                "C2002",
                Phase::Catalog,
                "CREATE TABLE AS SELECT is not supported in this iteration",
            ));
        }

        let table_name = normalize_object_name(&create_table.name, self.dialect);
        let original_table_name = original_object_name(&create_table.name);

        if catalog.table(&table_name).is_some() {
            if create_table.if_not_exists {
                return Ok(());
            }
            return Err(Diagnostic::new(
                "C2003",
                Phase::Catalog,
                format!("table '{}' already exists", original_table_name),
            ));
        }

        let mut columns = Vec::with_capacity(create_table.columns.len());
        for column_def in &create_table.columns {
            let column = self.build_column(column_def);
            if columns
                .iter()
                .any(|existing: &ColumnSchema| existing.name == column.name)
            {
                return Err(Diagnostic::new(
                    "C2004",
                    Phase::Catalog,
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
            self.apply_inline_column_constraints(&mut table, column_def)?;
        }

        for constraint in &create_table.constraints {
            match self.parse_table_constraint(catalog, &table, constraint)? {
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

        self.enforce_primary_key_nullability(&mut table);
        self.validate_foreign_keys(catalog, &table)?;

        catalog.add_table(table).map_err(|message| {
            Diagnostic::new(
                "C2005",
                Phase::Catalog,
                format!("failed to add table: {message}"),
            )
        })?;

        Ok(())
    }

    fn apply_alter_table(
        &self,
        catalog: &mut Catalog,
        table_name_ast: &sqlparser::ast::ObjectName,
        if_exists: bool,
        operations: &[AlterTableOperation],
    ) -> Result<(), Diagnostic> {
        let table_name = normalize_object_name(table_name_ast, self.dialect);
        let Some(table_index) = catalog.table_index(&table_name) else {
            if if_exists {
                return Ok(());
            }
            return Err(Diagnostic::new(
                "C2006",
                Phase::Catalog,
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
                    let column = self.build_column(column_def);
                    let table = &mut catalog.tables[table_index];
                    if table.has_column(&column.name) {
                        if *if_not_exists {
                            continue;
                        }
                        return Err(Diagnostic::new(
                            "C2007",
                            Phase::Catalog,
                            format!(
                                "column '{}' already exists in '{}'",
                                column.name, table.name
                            ),
                        ));
                    }
                    table.columns.push(column);
                    self.apply_inline_column_constraints(table, column_def)?;
                    self.enforce_primary_key_nullability(table);
                },
                AlterTableOperation::DropColumn {
                    column_name,
                    if_exists,
                    ..
                } => {
                    let normalized_column = normalize_ident(column_name, self.dialect);
                    self.validate_drop_column(catalog, table_index, &normalized_column)?;
                    let table = &mut catalog.tables[table_index];
                    let Some(column_index) = table.column_index(&normalized_column) else {
                        if *if_exists {
                            continue;
                        }
                        return Err(Diagnostic::new(
                            "C2008",
                            Phase::Catalog,
                            format!(
                                "column '{}' not found in table '{}'",
                                normalized_column, table.name
                            ),
                        ));
                    };
                    table.columns.remove(column_index);
                },
                AlterTableOperation::AddConstraint(constraint) => {
                    if matches!(self.dialect, Dialect::SQLite) {
                        return Err(Diagnostic::new(
                            "C2026",
                            Phase::Catalog,
                            "SQLite does not support ALTER TABLE ADD CONSTRAINT",
                        ));
                    }
                    self.apply_constraint_by_index(catalog, table_index, constraint)?;
                },
                _ => {
                    return Err(Diagnostic::new(
                        "C2009",
                        Phase::Catalog,
                        "unsupported ALTER TABLE operation in this iteration",
                    ));
                },
            }
        }

        Ok(())
    }

    fn apply_drop(
        &self,
        catalog: &mut Catalog,
        object_type: &ObjectType,
        if_exists: bool,
        names: &[sqlparser::ast::ObjectName],
    ) -> Result<(), Diagnostic> {
        if *object_type != ObjectType::Table {
            return Err(Diagnostic::new(
                "C2010",
                Phase::Catalog,
                "DROP only supports TABLE in this iteration",
            ));
        }

        for object_name in names {
            let normalized_name = normalize_object_name(object_name, self.dialect);
            if catalog.table(&normalized_name).is_none() {
                if if_exists {
                    continue;
                }
                return Err(Diagnostic::new(
                    "C2011",
                    Phase::Catalog,
                    format!("table '{}' does not exist", normalized_name),
                ));
            }

            if self.is_referenced_by_foreign_key(catalog, &normalized_name) {
                return Err(Diagnostic::new(
                    "C2012",
                    Phase::Catalog,
                    format!(
                        "cannot drop table '{}': referenced by foreign key constraints",
                        normalized_name
                    ),
                ));
            }

            let _ = catalog.drop_table(&normalized_name).map_err(|message| {
                Diagnostic::new(
                    "C2013",
                    Phase::Catalog,
                    format!("failed to drop table: {message}"),
                )
            })?;
        }

        Ok(())
    }

    fn build_column(&self, column_def: &ColumnDef) -> ColumnSchema {
        let name = normalize_ident(&column_def.name, self.dialect);
        let nullable = !column_def
            .options
            .iter()
            .any(|option_def| matches!(&option_def.option, ColumnOption::NotNull));

        ColumnSchema {
            name,
            original_name: column_def.name.value.clone(),
            data_type: ddl_type_map::map_sql_data_type(self.dialect, &column_def.data_type),
            nullable,
        }
    }

    fn apply_inline_column_constraints(
        &self,
        table: &mut TableSchema,
        column_def: &ColumnDef,
    ) -> Result<(), Diagnostic> {
        let column_name = normalize_ident(&column_def.name, self.dialect);
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
                            .map(|ident| normalize_ident(ident, self.dialect))
                            .collect()
                    };
                    table.foreign_keys.push(ForeignKeyConstraint {
                        name: option_def.name.as_ref().map(|ident| ident.value.clone()),
                        columns: vec![column_name.clone()],
                        ref_table: normalize_object_name(foreign_table, self.dialect),
                        ref_columns,
                    });
                },
                _ => {},
            }
        }
        Ok(())
    }

    fn apply_constraint_by_index(
        &self,
        catalog: &mut Catalog,
        table_index: usize,
        constraint: &TableConstraint,
    ) -> Result<(), Diagnostic> {
        let table_snapshot = catalog.tables[table_index].clone();
        let parsed_constraint =
            self.parse_table_constraint(catalog, &table_snapshot, constraint)?;

        {
            let table = &mut catalog.tables[table_index];
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
            self.enforce_primary_key_nullability(table);
        }

        let table_after_mutation = catalog.tables[table_index].clone();
        self.validate_foreign_keys(catalog, &table_after_mutation)?;
        Ok(())
    }

    fn parse_table_constraint(
        &self,
        catalog: &Catalog,
        table: &TableSchema,
        constraint: &TableConstraint,
    ) -> Result<ParsedTableConstraint, Diagnostic> {
        match constraint {
            TableConstraint::PrimaryKey { name, columns, .. } => {
                let normalized_columns =
                    self.normalize_and_validate_columns(table, columns, "primary key")?;
                Ok(ParsedTableConstraint::PrimaryKey(KeyConstraint {
                    name: name.as_ref().map(|ident| ident.value.clone()),
                    columns: normalized_columns,
                }))
            },
            TableConstraint::Unique { name, columns, .. } => {
                let normalized_columns =
                    self.normalize_and_validate_columns(table, columns, "unique key")?;
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
                    self.normalize_and_validate_columns(table, columns, "foreign key")?;
                let normalized_ref_columns = if referred_columns.is_empty() {
                    normalized_columns.clone()
                } else {
                    referred_columns
                        .iter()
                        .map(|ident| normalize_ident(ident, self.dialect))
                        .collect()
                };

                let foreign_key = ForeignKeyConstraint {
                    name: name.as_ref().map(|ident| ident.value.clone()),
                    columns: normalized_columns,
                    ref_table: normalize_object_name(foreign_table, self.dialect),
                    ref_columns: normalized_ref_columns,
                };

                let mut validation_table = table.clone();
                validation_table.foreign_keys.push(foreign_key.clone());
                self.validate_foreign_keys(catalog, &validation_table)?;

                Ok(ParsedTableConstraint::ForeignKey(foreign_key))
            },
            _ => Ok(ParsedTableConstraint::Unsupported),
        }
    }

    fn normalize_and_validate_columns(
        &self,
        table: &TableSchema,
        columns: &[sqlparser::ast::Ident],
        label: &str,
    ) -> Result<Vec<String>, Diagnostic> {
        if columns.is_empty() {
            return Err(Diagnostic::new(
                "C2014",
                Phase::Catalog,
                format!("{label} must contain at least one column"),
            ));
        }

        let mut normalized_columns = Vec::with_capacity(columns.len());
        for column in columns {
            let normalized = normalize_ident(column, self.dialect);
            if !table.has_column(&normalized) {
                return Err(Diagnostic::new(
                    "C2015",
                    Phase::Catalog,
                    format!(
                        "column '{}' not found in table '{}'",
                        normalized, table.name
                    ),
                ));
            }
            if normalized_columns.iter().any(|name| name == &normalized) {
                return Err(Diagnostic::new(
                    "C2016",
                    Phase::Catalog,
                    format!("column '{}' repeated in {}", normalized, label),
                ));
            }
            normalized_columns.push(normalized);
        }
        Ok(normalized_columns)
    }

    fn enforce_primary_key_nullability(&self, table: &mut TableSchema) {
        let Some(primary_key) = &table.primary_key else {
            return;
        };

        for column_name in &primary_key.columns {
            let Some(column_index) = table.column_index(column_name) else {
                continue;
            };

            let make_not_null = match self.dialect {
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

    fn validate_foreign_keys(
        &self,
        catalog: &Catalog,
        table: &TableSchema,
    ) -> Result<(), Diagnostic> {
        for foreign_key in &table.foreign_keys {
            if foreign_key.columns.is_empty() {
                return Err(Diagnostic::new(
                    "C2017",
                    Phase::Catalog,
                    format!("foreign key in '{}' has no local columns", table.name),
                ));
            }
            if foreign_key.columns.len() != foreign_key.ref_columns.len() {
                return Err(Diagnostic::new(
                    "C2018",
                    Phase::Catalog,
                    format!(
                        "foreign key in '{}' has mismatched local/referenced column counts",
                        table.name
                    ),
                ));
            }
            for local_column in &foreign_key.columns {
                if !table.has_column(local_column) {
                    return Err(Diagnostic::new(
                        "C2019",
                        Phase::Catalog,
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
                catalog.table(&foreign_key.ref_table)
            };

            let Some(referenced_table) = referenced_table else {
                return Err(Diagnostic::new(
                    "C2020",
                    Phase::Catalog,
                    format!(
                        "foreign key references unknown table '{}' from '{}'",
                        foreign_key.ref_table, table.name
                    ),
                ));
            };

            for ref_column in &foreign_key.ref_columns {
                if !referenced_table.has_column(ref_column) {
                    return Err(Diagnostic::new(
                        "C2021",
                        Phase::Catalog,
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

    fn validate_drop_column(
        &self,
        catalog: &Catalog,
        table_index: usize,
        column_name: &str,
    ) -> Result<(), Diagnostic> {
        let table = &catalog.tables[table_index];

        if table
            .primary_key
            .as_ref()
            .is_some_and(|key| key.columns.iter().any(|name| name == column_name))
        {
            return Err(Diagnostic::new(
                "C2022",
                Phase::Catalog,
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
            return Err(Diagnostic::new(
                "C2023",
                Phase::Catalog,
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
            return Err(Diagnostic::new(
                "C2024",
                Phase::Catalog,
                format!(
                    "cannot drop column '{}.{}': used by foreign key",
                    table.name, column_name
                ),
            ));
        }

        if catalog.tables.iter().any(|other_table| {
            other_table.foreign_keys.iter().any(|foreign_key| {
                foreign_key.ref_table == table.name
                    && foreign_key
                        .ref_columns
                        .iter()
                        .any(|ref_column| ref_column == column_name)
            })
        }) {
            return Err(Diagnostic::new(
                "C2025",
                Phase::Catalog,
                format!(
                    "cannot drop column '{}.{}': referenced by other foreign keys",
                    table.name, column_name
                ),
            ));
        }

        Ok(())
    }

    fn is_referenced_by_foreign_key(&self, catalog: &Catalog, table_name: &str) -> bool {
        catalog.tables.iter().any(|table| {
            table
                .foreign_keys
                .iter()
                .any(|foreign_key| foreign_key.ref_table == table_name)
        })
    }
}
