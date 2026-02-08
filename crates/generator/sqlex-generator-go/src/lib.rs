use std::path::{Path, PathBuf};

use anyhow::Result;
use async_trait::async_trait;
use sqlex_common::ir::CompilationUnit;
use sqlex_generator::Generator;

pub struct GoGenerator {
    #[allow(dead_code)]
    output: PathBuf,
}

impl GoGenerator {
    pub fn new(output: &Path) -> Self {
        Self {
            output: output.to_path_buf(),
        }
    }
}

#[async_trait]
impl Generator for GoGenerator {
    async fn generate(&self, _input: &CompilationUnit) -> Result<()> {
        todo!("Implement Go code generation")
    }
}
