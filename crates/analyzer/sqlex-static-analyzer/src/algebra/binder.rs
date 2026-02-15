use sqlex_common::dialect::Dialect;

#[derive(Debug, Default)]
pub(crate) struct BinderContext {
    pub(crate) dialect: Option<Dialect>,
    pub(crate) scope_depth: usize,
}

impl BinderContext {
    pub(crate) fn with_dialect(dialect: Dialect) -> Self {
        Self {
            dialect: Some(dialect),
            scope_depth: 0,
        }
    }

    pub(crate) fn push_scope(&mut self) {
        self.scope_depth += 1;
    }

    pub(crate) fn pop_scope(&mut self) {
        if self.scope_depth > 0 {
            self.scope_depth -= 1;
        }
    }
}
