use anyhow::Result;
use sqlex_common::{
    SqlexConfig,
    ir::{CompilationUnit, ParameterDescriptor, QueryDescriptor},
    types::{ColumnInfo, DataType, Table},
};
use sqlex_generator::Generator;
use sqlex_generator_console::ConsoleGenerator;

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
                    type_info: DataType::Int,
                    nullable: false,
                }],
                columns: vec![
                    ColumnInfo {
                        name: "id".to_string(),
                        data_type: DataType::Int,
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
        for gen_config in &self.config.generators {
            println!(
                "Running generator: {} ({})",
                gen_config.name, gen_config.generator
            );
            let generator: Box<dyn Generator> = match gen_config.generator.as_str() {
                "console" => Box::new(ConsoleGenerator::new(gen_config.config.clone())?),
                _ => {
                    println!("Unknown generator type: {}", gen_config.generator);
                    continue;
                },
            };

            let output = generator.generate(&compilation_unit)?;

            if gen_config.generator == "console" {
                println!("{}", output);
            } else {
                // write to file... logic would go here
                println!("(File writing not implemented for other generators yet)");
            }
        }

        Ok(())
    }
}
