use std::path::Path;

use anyhow::Result;
use sqlex_common::{
    config::SqlexConfig,
    ir::{CompilationUnit, ParameterDescriptor, QueryDescriptor},
    types::{ColumnInfo, DataType, Table},
};

mod factory;
mod project;

pub use project::Project;

pub struct Compiler {
    project: Project,
}

impl Compiler {
    pub async fn new(config: SqlexConfig, config_path: &Path) -> Result<Self> {
        let project = Project::build(config, config_path).await?;
        Ok(Self { project })
    }

    pub async fn compile(&self) -> Result<()> {
        println!("Compiler: Starting compilation process...");

        // 1. Scan Project
        println!("Step 1: Project scanned");
        println!("  Found {} migration(s)", self.project.migrations.len());
        println!("  Found {} query(ies)", self.project.queries.len());

        // 2. Initialize Analyzer
        // todo!()

        // 3. Analyze Queries
        // todo!()
        // Mock CompilationUnit for testing generator
        let compilation_unit = CompilationUnit {
            tables: vec![Table {
                name: "users".to_string(),
                columns: vec![
                    ColumnInfo {
                        name: "id".to_string(),
                        data_type: DataType::Int(false),
                        nullability: false,
                    },
                    ColumnInfo {
                        name: "name".to_string(),
                        data_type: DataType::Text,
                        nullability: false,
                    },
                    ColumnInfo {
                        name: "email".to_string(),
                        data_type: DataType::Text,
                        nullability: true,
                    },
                ],
            }],
            queries: vec![QueryDescriptor {
                name: "find_user_by_id".to_string(),
                sql: "SELECT * FROM users WHERE id = ?".to_string(),
                params: vec![ParameterDescriptor {
                    name: "id".to_string(),
                    type_info: DataType::Int(false),
                    nullable: false,
                }],
                columns: vec![
                    ColumnInfo {
                        name: "id".to_string(),
                        data_type: DataType::Int(false),
                        nullability: false,
                    },
                    ColumnInfo {
                        name: "name".to_string(),
                        data_type: DataType::Text,
                        nullability: false,
                    },
                    ColumnInfo {
                        name: "email".to_string(),
                        data_type: DataType::Text,
                        nullability: true,
                    },
                ],
            }],
        };

        // 4. Initialize & Run Generators
        for gen_config in &self.project.config.generators {
            println!(
                "Running generator: {} ({})",
                gen_config.name, gen_config.generator
            );
            let generator =
                factory::get_generator(&gen_config.generator, gen_config.config.clone()).map_err(
                    |e| anyhow::anyhow!("Failed to create generator '{}': {}", gen_config.name, e),
                )?;

            generator.generate(&compilation_unit).await?;

            println!("Generator {} finished.", gen_config.name);
        }

        Ok(())
    }
}
