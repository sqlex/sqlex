use std::collections::{HashMap, HashSet};

use crate::{
    algebraizer::model::{relation::ScanNode, schema::ColumnOrigin as BoundColumnOrigin},
    catalog::Catalog,
    infer::model::metadata::ResolvedKey,
};

pub(super) fn resolve_scan_keys(node: &ScanNode, catalog: &Catalog) -> Vec<ResolvedKey> {
    let Ok(table) = catalog.get_table(&node.table) else {
        return Vec::new();
    };

    let slot_by_column: HashMap<String, u32> = node
        .schema
        .columns
        .iter()
        .filter_map(|column| match &column.origin {
            BoundColumnOrigin::Base {
                table,
                column: name,
            } if table == &node.table => Some((name.clone(), column.slot_id)),
            _ => None,
        })
        .collect();

    let mut unique = HashSet::new();
    let mut keys = Vec::new();
    if let Some(primary_key) = &table.primary_key {
        if let Some(key) = key_from_column_names(&primary_key.columns, &slot_by_column) {
            let identity = key.slot_ids.clone();
            if unique.insert(identity) {
                keys.push(key);
            }
        }
    }
    for unique_key in &table.unique_keys {
        if let Some(key) = key_from_column_names(&unique_key.columns, &slot_by_column) {
            let identity = key.slot_ids.clone();
            if unique.insert(identity) {
                keys.push(key);
            }
        }
    }
    keys
}

fn key_from_column_names(
    column_names: &[String],
    slot_by_column: &HashMap<String, u32>,
) -> Option<ResolvedKey> {
    let mut slots = Vec::with_capacity(column_names.len());
    for column_name in column_names {
        let slot_id = slot_by_column.get(column_name)?;
        slots.push(*slot_id);
    }
    ResolvedKey::from_slots(slots)
}

pub(super) fn slots_key(slots: impl IntoIterator<Item = u32>) -> Vec<ResolvedKey> {
    let slots: Vec<u32> = slots.into_iter().collect();
    ResolvedKey::from_slots(slots).map_or_else(Vec::new, |key| vec![key])
}
