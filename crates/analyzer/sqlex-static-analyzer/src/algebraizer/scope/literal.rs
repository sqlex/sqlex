#[derive(Debug)]
pub struct LiteralAssignmentModeStack {
    stack: Vec<bool>,
}

impl LiteralAssignmentModeStack {
    pub fn new() -> Self {
        Self { stack: vec![false] }
    }

    pub fn push(&mut self) {
        let parent = *self.stack.last().expect("stack never empty");
        self.stack.push(parent);
    }

    pub fn push_with(&mut self, value: bool) {
        self.stack.push(value);
    }

    pub fn pop(&mut self) {
        if self.stack.len() <= 1 {
            panic!("cannot pop root literal assignment mode scope");
        }
        let _ = self.stack.pop();
    }

    pub fn current(&self) -> bool {
        *self
            .stack
            .last()
            .expect("literal assignment mode stack is never empty")
    }
}

impl Default for LiteralAssignmentModeStack {
    fn default() -> Self {
        Self::new()
    }
}
