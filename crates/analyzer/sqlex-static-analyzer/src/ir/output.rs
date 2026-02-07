use sqlex_common::types::{Cardinality, DataType};

#[derive(Debug, Clone)]
pub struct OutputSchema {
    pub columns: Vec<OutputColumn>,
    pub cardinality: Cardinality,
}

#[derive(Debug, Clone)]
pub struct OutputColumn {
    pub name: String,
    pub data_type: DataType,
    pub nullability: bool,
}
