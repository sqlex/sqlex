use anyhow::Result;
use sqlex_common::ir::CompilationUnit;
use sqlex_generator::Generator;

pub struct JavaGenerator;

impl JavaGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Generator for JavaGenerator {
    fn generate(&self, _input: &CompilationUnit) -> Result<String> {
        todo!("Implement Java code generation")
    }

    fn extension(&self) -> &str {
        "java"
    }
}
