use anyhow::Result;
use sqlex_common::ir::CompilationUnit;
use sqlex_generator::Generator;

pub struct JavaGenerator;

impl JavaGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JavaGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for JavaGenerator {
    fn generate(&self, _input: &CompilationUnit) -> Result<()> {
        todo!("Implement Java code generation")
    }
}
