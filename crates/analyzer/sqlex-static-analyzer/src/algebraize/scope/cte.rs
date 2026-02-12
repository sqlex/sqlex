use std::collections::HashMap;

use crate::algebraize::CteEntry;

#[derive(Debug, Default)]
pub(in crate::algebraize) struct CteScopes {
    scopes: Vec<HashMap<String, CteEntry>>,
}

impl CteScopes {
    pub(in crate::algebraize) fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    pub(in crate::algebraize) fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(in crate::algebraize) fn pop(&mut self) {
        if self.scopes.len() > 1 {
            let _ = self.scopes.pop();
        } else if let Some(scope) = self.scopes.last_mut() {
            scope.clear();
        }
    }

    pub(in crate::algebraize) fn insert(&mut self, name: String, entry: CteEntry) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, entry);
        }
    }

    pub(in crate::algebraize) fn contains_in_current(&self, name: &str) -> bool {
        self.scopes
            .last()
            .map(|scope| scope.contains_key(name))
            .unwrap_or(false)
    }

    pub(in crate::algebraize) fn get(&self, name: &str) -> Option<&CteEntry> {
        for scope in self.scopes.iter().rev() {
            if let Some(entry) = scope.get(name) {
                return Some(entry);
            }
        }
        None
    }
}
