use anyhow::Result;
use sqlex_common::ir::CompilationUnit;
use sqlex_generator::Generator;

pub struct RustGenerator;

impl RustGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RustGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for RustGenerator {
    fn generate(&self, _input: &CompilationUnit) -> Result<()> {
        todo!("Implement Rust code generation")
    }
}
