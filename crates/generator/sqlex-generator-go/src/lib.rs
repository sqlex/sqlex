use anyhow::Result;
use sqlex_common::ir::CompilationUnit;
use sqlex_generator::Generator;

pub struct GoGenerator;

impl GoGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GoGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for GoGenerator {
    fn generate(&self, _input: &CompilationUnit) -> Result<String> {
        todo!("Implement Go code generation")
    }

    fn extension(&self) -> &str {
        "go"
    }
}
