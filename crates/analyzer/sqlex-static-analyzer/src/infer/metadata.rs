use sqlex_common::types::{ColumnInfo, DataType, ResultSet};

use crate::infer::cardinality::{CardInterval, MaxRows, MinRows};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ColumnOrigin {
    Base { table: String, column: String },
    Derived,
}

#[derive(Debug, Clone)]
pub(crate) struct InferColumn {
    pub(crate) slot_id: Option<u32>,
    pub(crate) name: String,
    pub(crate) data_type: DataType,
    pub(crate) nullable: bool,
    pub(crate) origin: ColumnOrigin,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedKey {
    pub(crate) columns: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct InferMetadata {
    pub(crate) columns: Vec<InferColumn>,
    pub(crate) cardinality: CardInterval,
    pub(crate) keys: Vec<ResolvedKey>,
}

impl Default for InferMetadata {
    fn default() -> Self {
        Self {
            columns: Vec::new(),
            cardinality: CardInterval {
                min: MinRows::Zero,
                max: MaxRows::Many,
            },
            keys: Vec::new(),
        }
    }
}

impl InferMetadata {
    pub(crate) fn to_result_set(&self) -> ResultSet {
        ResultSet {
            columns: self
                .columns
                .iter()
                .map(|column| ColumnInfo {
                    name: column.name.clone(),
                    data_type: column.data_type.clone(),
                    nullability: column.nullable,
                })
                .collect(),
            cardinality: self.cardinality.to_cardinality(),
        }
    }
}
