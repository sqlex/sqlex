use std::collections::HashMap;

use crate::algebra::{expr::RelExpr, scalar::OutputSchema};

#[derive(Debug, Clone)]
pub(crate) struct RelationScope {
    pub(crate) visible_names: Vec<String>,
    pub(crate) schema: OutputSchema,
}

#[derive(Debug, Clone)]
pub(crate) struct CteBinding {
    pub(crate) expr: RelExpr,
    pub(crate) exposed_schema: OutputSchema,
}

#[derive(Debug)]
pub(crate) struct BuildContext {
    pub(crate) relation_scopes: Vec<RelationScope>,
    pub(crate) next_relation_id: u32,
    pub(crate) next_slot_id: u32,
    pub(crate) ctes: HashMap<String, CteBinding>,
    pub(crate) literal_assignment_mode: bool,
}

impl BuildContext {
    pub(crate) fn new() -> Self {
        Self {
            relation_scopes: Vec::new(),
            next_relation_id: 1,
            next_slot_id: 1,
            ctes: HashMap::new(),
            literal_assignment_mode: false,
        }
    }

    pub(crate) fn allocate_relation_id(&mut self) -> u32 {
        let relation_id = self.next_relation_id;
        self.next_relation_id += 1;
        relation_id
    }

    pub(crate) fn allocate_slot_id(&mut self) -> u32 {
        let slot_id = self.next_slot_id;
        self.next_slot_id += 1;
        slot_id
    }

    pub(crate) fn current_columns(&self) -> Vec<crate::algebra::scalar::BoundColumn> {
        self.relation_scopes
            .iter()
            .flat_map(|scope| scope.schema.columns.iter().cloned())
            .collect()
    }
}
