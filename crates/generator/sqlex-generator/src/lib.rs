use anyhow::Result;
use async_trait::async_trait;
use sqlex_common::ir::CompilationUnit;

mod file_writer;

pub use file_writer::FileWriter;

#[async_trait]
pub trait Generator: Send + Sync {
    /// Generate code from the compilation unit.
    async fn generate(&self, input: &CompilationUnit) -> Result<()>;
}
