use std::collections::{HashMap, HashSet};

use sqlparser::ast::WindowSpec;

use crate::algebraizer::model::{relation::Relation, schema::OutputSchema};

#[derive(Debug, Clone)]
pub(crate) struct RelationScope {
    pub(crate) visible_names: Vec<String>,
    pub(crate) schema: OutputSchema,
    pub(crate) hidden_unqualified_slots: HashSet<u32>,
}

#[derive(Debug, Clone)]
pub(crate) struct CteBinding {
    pub(crate) relation: Relation,
    pub(crate) exposed_schema: OutputSchema,
}

#[derive(Debug)]
pub(crate) struct BuildContext {
    pub(crate) relation_scopes: Vec<RelationScope>,
    pub(crate) outer_relation_scopes: Vec<Vec<RelationScope>>,
    pub(crate) next_relation_id: u32,
    pub(crate) next_slot_id: u32,
    pub(crate) ctes: HashMap<String, CteBinding>,
    pub(crate) named_windows: HashMap<String, WindowSpec>,
    pub(crate) literal_assignment_mode: bool,
}

impl BuildContext {
    pub(crate) fn new() -> Self {
        Self {
            relation_scopes: Vec::new(),
            outer_relation_scopes: Vec::new(),
            next_relation_id: 1,
            next_slot_id: 1,
            ctes: HashMap::new(),
            named_windows: HashMap::new(),
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
}
