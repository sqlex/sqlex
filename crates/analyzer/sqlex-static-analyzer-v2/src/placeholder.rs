use sqlex_common::types::DataType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placeholder {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
}
