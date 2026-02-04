use sqlex_common::types::DataType;

#[derive(Debug, Clone)]
pub struct OutputSchema {
    pub columns: Vec<OutputColumn>,
}

#[derive(Debug, Clone)]
pub struct OutputColumn {
    pub name: String,
    pub data_type: DataType,
    pub nullability: bool,
    pub lineage: Vec<LineageColumn>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LineageColumn {
    pub table: Option<String>,
    pub column: String,
}
