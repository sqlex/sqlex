use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlex_common::ir::CompilationUnit;
use sqlex_generator::Generator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleGeneratorConfig {
    #[serde(default)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Json,
    #[default]
    Text,
}

pub struct ConsoleGenerator {
    config: ConsoleGeneratorConfig,
}

impl ConsoleGenerator {
    pub fn new(config: serde_json::Value) -> Result<Self> {
        let config: ConsoleGeneratorConfig = serde_json::from_value(config)?;
        Ok(Self { config })
    }
}

impl Generator for ConsoleGenerator {
    fn generate(&self, input: &CompilationUnit) -> Result<String> {
        match self.config.format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(input)?;
                Ok(json)
            },
            OutputFormat::Text => {
                // Human readable text format
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

                Ok(output)
            },
        }
    }

    fn extension(&self) -> &str {
        match self.config.format {
            OutputFormat::Json => "json",
            OutputFormat::Text => "txt",
        }
    }
}
