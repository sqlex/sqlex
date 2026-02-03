mod factory;

use anyhow::Result;
use sqlex_common::{
    SqlexConfig,
    ir::{CompilationUnit, ParameterDescriptor, QueryDescriptor},
    types::{ColumnInfo, DataType, Table},
};

pub struct Compiler {
    #[allow(dead_code)]
    config: SqlexConfig,
}

impl Compiler {
    pub fn new(config: SqlexConfig) -> Self {
        Self { config }
    }

    pub async fn compile(&self) -> Result<()> {
        println!("Compiler: Starting compilation process...");

        // 1. Scan Project
        // todo!()

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
                        data_type: DataType::Int,
                        nullability: false,
                        origin_table: None,
                        origin_column: None,
                    },
                    ColumnInfo {
                        name: "name".to_string(),
                        data_type: DataType::Text,
                        nullability: false,
                        origin_table: None,
                        origin_column: None,
                    },
                    ColumnInfo {
                        name: "email".to_string(),
                        data_type: DataType::Text,
                        nullability: true,
                        origin_table: None,
                        origin_column: None,
                    },
                ],
            }],
            queries: vec![QueryDescriptor {
                name: "find_user_by_id".to_string(),
                sql: "SELECT * FROM users WHERE id = ?".to_string(),
                params: vec![ParameterDescriptor {
                    name: "id".to_string(),
                    type_info: DataType::Int,
                    nullable: false,
                }],
                columns: vec![
                    ColumnInfo {
                        name: "id".to_string(),
                        data_type: DataType::Int,
                        nullability: false,
                        origin_table: None,
                        origin_column: None,
                    },
                    ColumnInfo {
                        name: "name".to_string(),
                        data_type: DataType::Text,
                        nullability: false,
                        origin_table: None,
                        origin_column: None,
                    },
                    ColumnInfo {
                        name: "email".to_string(),
                        data_type: DataType::Text,
                        nullability: true,
                        origin_table: None,
                        origin_column: None,
                    },
                ],
            }],
        };

        // 4. Initialize & Run Generators
        for gen_config in &self.config.generators {
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
