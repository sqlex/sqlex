use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlex_common::ir::CompilationUnit;
use sqlex_generator::Generator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugGeneratorConfig {
    #[serde(default)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Json,
    Yaml,
    #[default]
    Text,
}

pub struct DebugGenerator {
    output: PathBuf,
    config: DebugGeneratorConfig,
}

impl DebugGenerator {
    pub fn new(output: &Path, config: serde_json::Value) -> Result<Self> {
        let config: DebugGeneratorConfig = serde_json::from_value(config)?;
        Ok(Self {
            output: output.to_path_buf(),
            config,
        })
    }

    fn format_text(input: &CompilationUnit) -> String {
        let mut output = String::new();

        output.push_str("=== Compilation Unit ===\n\n");

        output.push_str("--- Tables ---\n");
        for table in &input.tables {
            output.push_str(&format!("Table: {}\n", table.name));
            for col in &table.columns {
                output.push_str(&format!(
                    "  - {}: {:?} (Nullable: {})\n",
                    col.name, col.data_type, col.nullability
                ));
            }
            output.push('\n');
        }

        output.push_str("--- Queries ---\n");
        for query in &input.queries {
            output.push_str(&format!("Query: {}\n", query.name));
            output.push_str(&format!("  Package: {}\n", query.package.join(".")));
            output.push_str(&format!("  Module: {}\n", query.module));
            output.push_str(&format!("  Cardinality: {:?}\n", query.cardinality));
            output.push_str("  SQL:\n");
            for line in query.sql.lines() {
                output.push_str(&format!("    {}\n", line));
            }
            output.push_str("  Params:\n");
            for param in &query.params {
                output.push_str(&format!(
                    "    - {}: {:?} (Nullable: {})\n",
                    param.name, param.type_info, param.nullable
                ));
            }
            output.push_str("  Columns:\n");
            for col in &query.columns {
                output.push_str(&format!(
                    "    - {}: {:?} (Nullable: {})\n",
                    col.name, col.data_type, col.nullability
                ));
            }
            output.push('\n');
        }
        output
    }
}

#[async_trait]
impl Generator for DebugGenerator {
    async fn generate(&self, input: &CompilationUnit) -> Result<()> {
        let content = match self.config.format {
            OutputFormat::Json => serde_json::to_string_pretty(input)?,
            OutputFormat::Yaml => serde_yaml::to_string(input)?,
            OutputFormat::Text => Self::format_text(input),
        };

        if let Some(parent) = self.output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.output, content)?;

        Ok(())
    }
}
