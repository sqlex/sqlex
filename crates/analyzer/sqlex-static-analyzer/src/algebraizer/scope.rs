use std::collections::{HashMap, HashSet};

use sqlparser::ast::WindowSpec;

use crate::algebraizer::{
    Algebraizer,
    model::{relation::Relation, schema::OutputSchema},
};

pub(super) type CteScope = HashMap<String, CteBinding>;

#[derive(Debug, Clone)]
pub(crate) struct RelationBinding {
    pub(crate) qualifier_names: Vec<String>,
    pub(crate) schema: OutputSchema,
    pub(crate) hidden_unqualified_slot_ids: HashSet<u32>,
}

#[derive(Debug, Clone)]
pub(crate) struct CteBinding {
    pub(crate) relation: Relation,
    pub(crate) exposed_schema: OutputSchema,
}

#[derive(Debug, Clone)]
pub(super) struct QueryScope {
    pub(super) relation_bindings: Vec<RelationBinding>,
    pub(super) named_windows: HashMap<String, WindowSpec>,
    pub(super) literal_assignment_mode: bool,
}

impl Algebraizer<'_> {
    pub(crate) fn push_query_scope(&mut self, literal_assignment_mode: bool) {
        self.query_scope_stack.push(QueryScope {
            relation_bindings: Vec::new(),
            named_windows: HashMap::new(),
            literal_assignment_mode,
        });
    }

    pub(crate) fn pop_query_scope(&mut self) {
        if self.query_scope_stack.len() <= 1 {
            panic!("cannot pop root query scope");
        }
        let _ = self.query_scope_stack.pop();
    }

    pub(crate) fn current_relation_bindings(&self) -> &[RelationBinding] {
        self.query_scope_stack
            .last()
            .expect("query scope stack is never empty")
            .relation_bindings
            .as_slice()
    }

    pub(crate) fn set_current_relation_bindings(&mut self, bindings: Vec<RelationBinding>) {
        self.query_scope_stack
            .last_mut()
            .expect("query scope stack is never empty")
            .relation_bindings = bindings;
    }

    pub(crate) fn iter_outer_query_relation_bindings(
        &self,
    ) -> impl Iterator<Item = &[RelationBinding]> {
        self.query_scope_stack[..self.query_scope_stack.len() - 1]
            .iter()
            .rev()
            .map(|scope| scope.relation_bindings.as_slice())
    }

    pub(crate) fn current_named_windows(&self) -> &HashMap<String, WindowSpec> {
        &self
            .query_scope_stack
            .last()
            .expect("query scope stack is never empty")
            .named_windows
    }

    pub(crate) fn take_current_named_windows(&mut self) -> HashMap<String, WindowSpec> {
        std::mem::take(
            &mut self
                .query_scope_stack
                .last_mut()
                .expect("query scope stack is never empty")
                .named_windows,
        )
    }

    pub(crate) fn set_current_named_windows(&mut self, windows: HashMap<String, WindowSpec>) {
        self.query_scope_stack
            .last_mut()
            .expect("query scope stack is never empty")
            .named_windows = windows;
    }

    pub(crate) fn literal_assignment_mode(&self) -> bool {
        self.query_scope_stack
            .last()
            .expect("query scope stack is never empty")
            .literal_assignment_mode
    }

    pub(crate) fn push_cte_scope(&mut self) {
        self.cte_scope_stack.push(HashMap::new());
    }

    pub(crate) fn pop_cte_scope(&mut self) {
        if self.cte_scope_stack.len() <= 1 {
            panic!("cannot pop root CTE scope");
        }
        let _ = self.cte_scope_stack.pop();
    }

    pub(crate) fn resolve_cte(&self, name: &str) -> Option<&CteBinding> {
        self.cte_scope_stack
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
    }

    pub(crate) fn insert_cte(&mut self, name: String, binding: CteBinding) -> Option<CteBinding> {
        self.cte_scope_stack
            .last_mut()
            .expect("CTE scope stack is never empty")
            .insert(name, binding)
    }

    pub(crate) fn cte_exists_in_any_scope(&self, name: &str) -> bool {
        self.cte_scope_stack
            .iter()
            .any(|scope| scope.contains_key(name))
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
