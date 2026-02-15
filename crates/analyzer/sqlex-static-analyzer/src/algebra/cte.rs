use std::collections::HashMap;

use crate::algebra::scalar::OutputSchema;

#[derive(Debug, Default)]
pub(crate) struct CteRegistry {
    ctes: HashMap<String, OutputSchema>,
}

impl CteRegistry {
    pub(crate) fn insert(&mut self, name: String, schema: OutputSchema) -> Option<OutputSchema> {
        self.ctes.insert(name, schema)
    }

    pub(crate) fn get(&self, name: &str) -> Option<&OutputSchema> {
        self.ctes.get(name)
    }
}
