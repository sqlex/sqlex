use std::collections::HashMap;

use crate::algebraizer::model::{relation::Relation, schema::OutputSchema};

#[derive(Debug, Clone)]
pub struct CteBinding {
    pub relation: Relation,
    pub exposed_schema: OutputSchema,
}

#[derive(Debug)]
pub struct CteScopeStack {
    stack: Vec<HashMap<String, CteBinding>>,
}

impl CteScopeStack {
    pub fn new() -> Self {
        Self {
            stack: vec![HashMap::new()],
        }
    }

    pub fn push(&mut self) {
        self.stack.push(HashMap::new());
    }

    pub fn pop(&mut self) {
        if self.stack.len() <= 1 {
            panic!("cannot pop root CTE scope");
        }
        let _ = self.stack.pop();
    }

    pub fn resolve(&self, name: &str) -> Option<&CteBinding> {
        self.stack.iter().rev().find_map(|scope| scope.get(name))
    }

    pub fn register(&mut self, name: String, binding: CteBinding) {
        self.stack
            .last_mut()
            .expect("CTE scope stack is never empty")
            .insert(name, binding);
    }

    pub fn exists_in_any_scope(&self, name: &str) -> bool {
        self.stack.iter().any(|scope| scope.contains_key(name))
    }
}

impl Default for CteScopeStack {
    fn default() -> Self {
        Self::new()
    }
}
