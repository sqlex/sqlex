use sqlex_common::types::DataType;

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
    #[allow(dead_code)]
    pub(crate) original_name: String,
    pub(crate) data_type: DataType,
    pub(crate) nullable: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct KeyConstraint {
    #[allow(dead_code)]
    pub(crate) name: Option<String>,
    pub(crate) columns: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ForeignKeyConstraint {
    #[allow(dead_code)]
    pub(crate) name: Option<String>,
    pub(crate) columns: Vec<String>,
    pub(crate) ref_table: String,
    pub(crate) ref_columns: Vec<String>,
}
