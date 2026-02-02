use anyhow::Result;
use sqlex_common::ir::CompilationUnit;

pub trait Generator: Send + Sync {
    /// Generate code from the compilation unit.
    fn generate(&self, input: &CompilationUnit) -> Result<String>;

    /// The file extension for the generated code (e.g., "rs", "ts").
    fn extension(&self) -> &str;
}
