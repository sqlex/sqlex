//! Schema registry implementation.

use std::collections::HashMap;

use sqlex_parser::{
    convert_data_type, parse, sqlparser, AlterColumnOperation, AlterTableOperation, ColumnOption,
    CreateTable, ObjectName, ObjectType, Statement, TableConstraint,
};
use sqlex_types::{ColumnDef, Dialect, SqlType, TableDef};

use crate::SchemaError;

/// A registry of table definitions representing database schema.
#[derive(Debug, Clone)]
pub struct SchemaRegistry {
    /// Map of table name (lowercase) to table definition
    tables: HashMap<String, TableDef>,
    /// The SQL dialect
    dialect: Dialect,
}

impl SchemaRegistry {
    /// Create a new empty schema registry.
    pub fn new(dialect: Dialect) -> Self {
        Self {
            tables: HashMap::new(),
            dialect,
        }
    }

    /// Get the dialect of this registry.
    pub fn dialect(&self) -> Dialect {
        self.dialect
    }

    /// Apply a SQL statement to update the schema.
    pub fn apply_statement(&mut self, stmt: &Statement) -> Result<(), SchemaError> {
        match stmt {
            Statement::CreateTable(create) => self.apply_create_table(create),
            Statement::Drop { object_type, names, if_exists, .. } => {
                self.apply_drop(object_type, names, *if_exists)
            }
            Statement::AlterTable { name, operations, .. } => {
                self.apply_alter_table(name, operations)
            }
            // Ignore non-DDL statements
            _ => Ok(()),
        }
    }

    /// Apply multiple SQL statements from a SQL string.
    pub fn apply_sql(&mut self, sql: &str) -> Result<(), SchemaError> {
        let statements = parse(sql, self.dialect)?;
        for stmt in &statements {
            self.apply_statement(stmt)?;
        }
        Ok(())
    }

    /// Get a table by name (case-insensitive).
    pub fn get_table(&self, name: &str) -> Option<&TableDef> {
        self.tables.get(&name.to_lowercase())
    }

    /// Get a mutable table by name.
    pub fn get_table_mut(&mut self, name: &str) -> Option<&mut TableDef> {
        self.tables.get_mut(&name.to_lowercase())
    }

    /// Check if a table exists.
    pub fn has_table(&self, name: &str) -> bool {
        self.tables.contains_key(&name.to_lowercase())
    }

    /// Get all tables.
    pub fn tables(&self) -> impl Iterator<Item = &TableDef> {
        self.tables.values()
    }

    /// Get the number of tables.
    pub fn table_count(&self) -> usize {
        self.tables.len()
    }

    // --- Private methods ---

    fn apply_create_table(&mut self, create: &CreateTable) -> Result<(), SchemaError> {
        let table_name = object_name_to_string(&create.name);
        let key = table_name.to_lowercase();

        if !create.or_replace && !create.if_not_exists && self.tables.contains_key(&key) {
            return Err(SchemaError::TableAlreadyExists(table_name));
        }

        let mut table_def = TableDef::new(&table_name);

        // Extract schema if present
        if create.name.0.len() > 1 {
            let parts: Vec<_> = create.name.0.iter().map(|i| ident_to_string(i)).collect();
            table_def.schema = Some(parts[..parts.len() - 1].join("."));
        }

        // Process columns
        for col in &create.columns {
            let col_def = convert_column_def(col, self.dialect);
            table_def.add_column(col_def);
        }

        // Process table constraints
        for constraint in &create.constraints {
            self.apply_table_constraint(&mut table_def, constraint);
        }

        self.tables.insert(key, table_def);
        Ok(())
    }

    fn apply_drop(
        &mut self,
        object_type: &ObjectType,
        names: &[ObjectName],
        if_exists: bool,
    ) -> Result<(), SchemaError> {
        if !matches!(object_type, ObjectType::Table) {
            return Ok(()); // Ignore non-table drops
        }

        for name in names {
            let table_name = object_name_to_string(name);
            let key = table_name.to_lowercase();

            if !if_exists && !self.tables.contains_key(&key) {
                return Err(SchemaError::TableNotFound(table_name));
            }

            self.tables.remove(&key);
        }
        Ok(())
    }

    fn apply_alter_table(
        &mut self,
        name: &ObjectName,
        operations: &[AlterTableOperation],
    ) -> Result<(), SchemaError> {
        let table_name = object_name_to_string(name);
        let key = table_name.to_lowercase();

        let table = self
            .tables
            .get_mut(&key)
            .ok_or_else(|| SchemaError::TableNotFound(table_name.clone()))?;

        for op in operations {
            match op {
                AlterTableOperation::AddColumn { column_def, if_not_exists, .. } => {
                    let col_name = column_def.name.value.clone();
                    if table.get_column(&col_name).is_some() {
                        if !if_not_exists {
                            return Err(SchemaError::ColumnAlreadyExists {
                                table: table_name.clone(),
                                column: col_name,
                            });
                        }
                    } else {
                        let col_def = convert_column_def(column_def, self.dialect);
                        table.add_column(col_def);
                    }
                }
                AlterTableOperation::DropColumn { column_name, if_exists, .. } => {
                    let col_name = column_name.value.clone();
                    if table.remove_column(&col_name).is_none() && !if_exists {
                        return Err(SchemaError::ColumnNotFound {
                            table: table_name.clone(),
                            column: col_name,
                        });
                    }
                }
                AlterTableOperation::RenameColumn { old_column_name, new_column_name } => {
                    let old_name = old_column_name.value.clone();
                    if let Some(col) = table.get_column_mut(&old_name) {
                        col.name = new_column_name.value.clone();
                    }
                }
                AlterTableOperation::AlterColumn { column_name, op } => {
                    let col_name = column_name.value.clone();
                    if let Some(col) = table.get_column_mut(&col_name) {
                        match op {
                            AlterColumnOperation::SetNotNull => {
                                col.nullable = false;
                            }
                            AlterColumnOperation::DropNotNull => {
                                col.nullable = true;
                            }
                            AlterColumnOperation::SetDataType { data_type, .. } => {
                                col.data_type = convert_data_type(data_type, self.dialect);
                            }
                            AlterColumnOperation::SetDefault { value } => {
                                col.default = Some(value.to_string());
                            }
                            AlterColumnOperation::DropDefault => {
                                col.default = None;
                            }
                            _ => {}
                        }
                    }
                }
                AlterTableOperation::RenameTable { table_name: new_name } => {
                    let new_table_name = object_name_to_string(new_name);
                    table.name = new_table_name.clone();
                    // Note: We'd need to update the key too, but that's complex
                    // For now, just update the name in the definition
                }
                _ => {
                    // Ignore other operations
                }
            }
        }
        Ok(())
    }

}

fn convert_column_def(col: &sqlex_parser::SqlColumnDef, dialect: Dialect) -> ColumnDef {
    let name = col.name.value.clone();
    let data_type = convert_data_type(&col.data_type, dialect);

    let mut col_def = ColumnDef::new(name, data_type);

    // Process column options
    for opt in &col.options {
        match &opt.option {
            ColumnOption::Null => {
                col_def.nullable = true;
            }
            ColumnOption::NotNull => {
                col_def.nullable = false;
            }
            ColumnOption::Default(expr) => {
                col_def.default = Some(expr.to_string());
            }
            ColumnOption::Unique { is_primary, .. } => {
                if *is_primary {
                    col_def.is_primary_key = true;
                    col_def.nullable = false;
                }
            }
            _ => {}
        }
    }

    col_def
}

impl SchemaRegistry {

    fn apply_table_constraint(&self, table: &mut TableDef, constraint: &TableConstraint) {
        match constraint {
            TableConstraint::PrimaryKey { columns, .. } => {
                for col in columns {
                    let col_name = col.value.clone();
                    table.primary_key.push(col_name.clone());
                    if let Some(col_def) = table.get_column_mut(&col_name) {
                        col_def.is_primary_key = true;
                        col_def.nullable = false;
                    }
                }
            }
            _ => {}
        }
    }
}

fn object_name_to_string(name: &ObjectName) -> String {
    name.0.last().map(|i| ident_to_string(i)).unwrap_or_default()
}

fn ident_to_string(ident: &sqlparser::ast::ObjectNamePart) -> String {
    match ident {
        sqlparser::ast::ObjectNamePart::Identifier(id) => id.value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_table() {
        let mut registry = SchemaRegistry::new(Dialect::PostgreSQL);
        registry
            .apply_sql("CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR(100) NOT NULL)")
            .unwrap();

        let table = registry.get_table("users").unwrap();
        assert_eq!(table.name, "users");
        assert_eq!(table.columns.len(), 2);

        let id_col = table.get_column("id").unwrap();
        assert_eq!(id_col.data_type, SqlType::Integer);
        assert!(id_col.is_primary_key);
        assert!(!id_col.nullable);

        let name_col = table.get_column("name").unwrap();
        assert_eq!(name_col.data_type, SqlType::Varchar(Some(100)));
        assert!(!name_col.nullable);
    }

    #[test]
    fn test_drop_table() {
        let mut registry = SchemaRegistry::new(Dialect::PostgreSQL);
        registry.apply_sql("CREATE TABLE users (id INT)").unwrap();
        assert!(registry.has_table("users"));

        registry.apply_sql("DROP TABLE users").unwrap();
        assert!(!registry.has_table("users"));
    }

    #[test]
    fn test_alter_table_add_column() {
        let mut registry = SchemaRegistry::new(Dialect::PostgreSQL);
        registry.apply_sql("CREATE TABLE users (id INT)").unwrap();
        registry
            .apply_sql("ALTER TABLE users ADD COLUMN name VARCHAR(100)")
            .unwrap();

        let table = registry.get_table("users").unwrap();
        assert_eq!(table.columns.len(), 2);
        assert!(table.get_column("name").is_some());
    }

    #[test]
    fn test_alter_table_drop_column() {
        let mut registry = SchemaRegistry::new(Dialect::PostgreSQL);
        registry
            .apply_sql("CREATE TABLE users (id INT, name VARCHAR(100))")
            .unwrap();
        registry
            .apply_sql("ALTER TABLE users DROP COLUMN name")
            .unwrap();

        let table = registry.get_table("users").unwrap();
        assert_eq!(table.columns.len(), 1);
        assert!(table.get_column("name").is_none());
    }

    #[test]
    fn test_case_insensitive() {
        let mut registry = SchemaRegistry::new(Dialect::PostgreSQL);
        registry.apply_sql("CREATE TABLE Users (id INT)").unwrap();

        assert!(registry.get_table("users").is_some());
        assert!(registry.get_table("USERS").is_some());
        assert!(registry.get_table("Users").is_some());
    }
}
