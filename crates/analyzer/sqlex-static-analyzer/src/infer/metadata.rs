use std::collections::HashMap;

use sqlex_common::types::{ColumnInfo, DataType, ResultSet};

use crate::infer::cardinality::CardInterval;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ResolvedKey {
    pub(crate) slot_ids: Vec<u32>,
}

impl ResolvedKey {
    pub(crate) fn from_slots(slot_ids: Vec<u32>) -> Option<Self> {
        if slot_ids.is_empty() {
            return None;
        }

        let mut normalized = slot_ids;
        normalized.sort_unstable();
        normalized.dedup();
        if normalized.is_empty() {
            return None;
        }

        Some(Self {
            slot_ids: normalized,
        })
    }
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
            cardinality: CardInterval::zero_or_more(),
            keys: Vec::new(),
        }
    }
}

impl InferMetadata {
    pub(crate) fn remap_keys(&self, slot_mapping: &HashMap<u32, u32>) -> Vec<ResolvedKey> {
        self.keys
            .iter()
            .filter_map(|key| {
                let mut mapped = Vec::with_capacity(key.slot_ids.len());
                for slot_id in &key.slot_ids {
                    let mapped_slot = slot_mapping.get(slot_id)?;
                    mapped.push(*mapped_slot);
                }
                ResolvedKey::from_slots(mapped)
            })
            .collect()
    }

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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::infer::metadata::{InferMetadata, ResolvedKey};

    #[test]
    fn remap_keys_drops_incomplete_mapping() {
        let metadata = InferMetadata {
            columns: Vec::new(),
            cardinality: crate::infer::cardinality::CardInterval::zero_or_more(),
            keys: vec![
                ResolvedKey::from_slots(vec![1]).expect("key should be created"),
                ResolvedKey::from_slots(vec![2, 3]).expect("key should be created"),
            ],
        };

        let mapping = HashMap::from([(1_u32, 10_u32), (2_u32, 20_u32)]);
        let remapped = metadata.remap_keys(&mapping);

        assert_eq!(
            remapped,
            vec![ResolvedKey::from_slots(vec![10]).expect("key should be created")]
        );
    }
}
