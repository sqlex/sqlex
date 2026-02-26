use crate::infer::model::metadata::InferColumn;

/// Stack of correlated outer scopes used by inference.
///
/// The stack stores one scope per correlated boundary:
/// the nearest outer relation is at the end of the vector.
#[derive(Debug, Clone, Default)]
pub(in crate::infer) struct OuterScopeStack {
    scopes: Vec<Vec<InferColumn>>,
}

impl OuterScopeStack {
    pub(in crate::infer) fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    pub(in crate::infer) fn push(&mut self, columns: &[InferColumn]) {
        self.scopes.push(columns.to_vec());
    }

    pub(in crate::infer) fn pop(&mut self) {
        let _ = self
            .scopes
            .pop()
            .expect("outer scope stack underflow during inference");
    }

    pub(in crate::infer) fn resolve(&self, depth: usize, slot_id: u32) -> Option<&InferColumn> {
        if depth == 0 || depth > self.scopes.len() {
            return None;
        }

        let scope_index = self.scopes.len() - depth;
        self.scopes[scope_index]
            .iter()
            .find(|column| column.slot_id.is_some_and(|value| value == slot_id))
    }

    pub(in crate::infer) fn len(&self) -> usize {
        self.scopes.len()
    }
}
