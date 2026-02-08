use std::path::Path;

use anyhow::{Result, anyhow};
use sqlex_generator::Generator;
use sqlex_generator_debug::DebugGenerator;
use sqlex_generator_go::GoGenerator;
use sqlex_generator_java::JavaGenerator;
use sqlex_generator_rust::RustGenerator;

pub fn get_generator(
    name: &str,
    output: &Path,
    config: serde_json::Value,
) -> Result<Box<dyn Generator>> {
    match name {
        "debug" => Ok(Box::new(DebugGenerator::new(output, config)?)),
        "go" => Ok(Box::new(GoGenerator::new(output))),
        "java" => Ok(Box::new(JavaGenerator::new(output))),
        "rust" => Ok(Box::new(RustGenerator::new(output))),
        _ => Err(anyhow!("Unknown generator type: {}", name)),
    }
}
