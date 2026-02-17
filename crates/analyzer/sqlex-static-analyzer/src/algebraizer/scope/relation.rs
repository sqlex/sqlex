use std::collections::HashSet;

use crate::algebraizer::model::schema::OutputSchema;

#[derive(Debug, Clone)]
pub struct RelationBinding {
    pub qualifier_names: Vec<String>,
    pub schema: OutputSchema,
    pub hidden_unqualified_slot_ids: HashSet<u32>,
}

#[derive(Debug)]
pub struct RelationScopeStack {
    stack: Vec<Vec<RelationBinding>>,
}

impl RelationScopeStack {
    pub fn new() -> Self {
        Self {
            stack: vec![Vec::new()],
        }
    }

    pub fn push(&mut self) {
        self.stack.push(Vec::new());
    }

    pub fn pop(&mut self) {
        if self.stack.len() <= 1 {
            panic!("cannot pop root relation scope");
        }
        let _ = self.stack.pop();
    }

    pub fn current(&self) -> &[RelationBinding] {
        self.stack
            .last()
            .expect("relation scope stack is never empty")
    }

    pub fn set_current(&mut self, bindings: Vec<RelationBinding>) {
        let last = self
            .stack
            .last_mut()
            .expect("relation scope stack is never empty");
        *last = bindings;
    }

    pub fn iter_outer(&self) -> impl Iterator<Item = &[RelationBinding]> {
        let len = self.stack.len();
        self.stack[..len - 1]
            .iter()
            .rev()
            .map(|scope| scope.as_slice())
    }
}

impl Default for RelationScopeStack {
    fn default() -> Self {
        Self::new()
    }
}
