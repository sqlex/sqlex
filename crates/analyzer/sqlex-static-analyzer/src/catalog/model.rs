use sqlex_common::types::{ColumnInfo, DataType, Table};

#[derive(Debug, Clone, Default)]
pub(crate) struct Catalog {
    pub(crate) tables: Vec<TableSchema>,
}

impl Catalog {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn table_index(&self, table_name: &str) -> Option<usize> {
        self.tables
            .iter()
            .position(|table| table.name == table_name)
    }

    pub(crate) fn table(&self, table_name: &str) -> Option<&TableSchema> {
        self.tables.iter().find(|table| table.name == table_name)
    }

    pub(crate) fn table_mut(&mut self, table_name: &str) -> Option<&mut TableSchema> {
        self.tables
            .iter_mut()
            .find(|table| table.name == table_name)
    }

    pub(crate) fn add_table(&mut self, table: TableSchema) -> Result<(), String> {
        if self.table(&table.name).is_some() {
            return Err(format!("table '{}' already exists", table.original_name));
        }
        self.tables.push(table);
        Ok(())
    }

    pub(crate) fn drop_table(&mut self, table_name: &str) -> Result<TableSchema, String> {
        let Some(index) = self.table_index(table_name) else {
            return Err(format!("table '{}' does not exist", table_name));
        };
        Ok(self.tables.remove(index))
    }

    pub(crate) fn to_tables(&self) -> Vec<Table> {
        self.tables
            .iter()
            .map(|table| Table {
                name: table.name.clone(),
                columns: table
                    .columns
                    .iter()
                    .map(|column| ColumnInfo {
                        name: column.name.clone(),
                        data_type: column.data_type.clone(),
                        nullability: column.nullable,
                    })
                    .collect(),
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TableSchema {
    pub(crate) name: String,
    pub(crate) original_name: String,
    pub(crate) columns: Vec<ColumnSchema>,
    pub(crate) primary_key: Option<KeyConstraint>,
    pub(crate) unique_keys: Vec<KeyConstraint>,
    pub(crate) foreign_keys: Vec<ForeignKeyConstraint>,
}

impl TableSchema {
    pub(crate) fn has_column(&self, column_name: &str) -> bool {
        self.columns.iter().any(|column| column.name == column_name)
    }

    pub(crate) fn column_index(&self, column_name: &str) -> Option<usize> {
        self.columns
            .iter()
            .position(|column| column.name == column_name)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ColumnSchema {
    pub(crate) name: String,
    pub(crate) original_name: String,
    pub(crate) data_type: DataType,
    pub(crate) nullable: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct KeyConstraint {
    pub(crate) name: Option<String>,
    pub(crate) columns: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ForeignKeyConstraint {
    pub(crate) name: Option<String>,
    pub(crate) columns: Vec<String>,
    pub(crate) ref_table: String,
    pub(crate) ref_columns: Vec<String>,
}
