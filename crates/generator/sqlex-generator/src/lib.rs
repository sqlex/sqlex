use anyhow::Result;
use sqlex_common::ir::CompilationUnit;

pub trait Generator: Send + Sync {
    /// Generate code from the compilation unit.
    fn generate(&self, input: &CompilationUnit) -> Result<()>;
}
