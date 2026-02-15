use sqlex_common::types::DataType;

use crate::algebraizer::model::expression::Expression;

pub(crate) type SlotId = u32;
pub(crate) type RelationId = u32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ColumnOrigin {
    Base { table: String, column: String },
    Derived,
}

#[derive(Debug, Clone)]
pub(crate) struct BoundColumn {
    pub(crate) slot_id: SlotId,
    pub(crate) name: String,
    #[allow(dead_code)]
    pub(crate) table_alias: Option<String>,
    pub(crate) data_type: Option<DataType>,
    pub(crate) nullable: bool,
    pub(crate) origin: ColumnOrigin,
}

#[derive(Debug, Clone)]
pub(crate) struct OutputSchema {
    #[allow(dead_code)]
    pub(crate) relation_id: RelationId,
    pub(crate) columns: Vec<BoundColumn>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Visibility {
    Visible,
    Hidden,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectionColumn {
    pub(crate) expr: Expression,
    pub(crate) alias: Option<String>,
    #[allow(dead_code)]
    pub(crate) visibility: Visibility,
}

#[derive(Debug, Clone)]
pub(crate) struct SortKey {
    pub(crate) expr: Expression,
    #[allow(dead_code)]
    pub(crate) asc: bool,
    #[allow(dead_code)]
    pub(crate) nulls_first: Option<bool>,
}
