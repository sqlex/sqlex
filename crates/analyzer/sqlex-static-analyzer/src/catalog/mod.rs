use sqlex_common::types::{ColumnInfo, Table};

pub(crate) mod model;
pub(crate) mod mutator;

#[derive(Debug, Clone, Default)]
pub(crate) struct Catalog {
    pub(crate) tables: Vec<model::TableSchema>,
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

    pub(crate) fn table(&self, table_name: &str) -> Option<&model::TableSchema> {
        self.tables.iter().find(|table| table.name == table_name)
    }

    pub(crate) fn add_table(&mut self, table: model::TableSchema) -> Result<(), String> {
        if self.table(&table.name).is_some() {
            return Err(format!("table '{}' already exists", table.original_name));
        }
        self.tables.push(table);
        Ok(())
    }

    pub(crate) fn drop_table(&mut self, table_name: &str) -> Result<model::TableSchema, String> {
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
