use std::collections::HashMap;

use sqlparser::ast::WindowSpec;

#[derive(Debug)]
pub struct NamedWindowScopeStack {
    stack: Vec<HashMap<String, WindowSpec>>,
}

impl NamedWindowScopeStack {
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
            panic!("cannot pop root named window scope");
        }
        let _ = self.stack.pop();
    }

    pub fn resolve(&self, name: &str) -> Option<&WindowSpec> {
        self.stack.iter().rev().find_map(|scope| scope.get(name))
    }

    pub fn register(&mut self, name: String, spec: WindowSpec) {
        self.stack
            .last_mut()
            .expect("named window scope stack is never empty")
            .insert(name, spec);
    }

    pub fn set_current(&mut self, windows: HashMap<String, WindowSpec>) {
        let last = self
            .stack
            .last_mut()
            .expect("named window scope stack is never empty");
        *last = windows;
    }

    pub fn current(&self) -> &HashMap<String, WindowSpec> {
        self.stack
            .last()
            .expect("named window scope stack is never empty")
    }
}

impl Default for NamedWindowScopeStack {
    fn default() -> Self {
        Self::new()
    }
}
