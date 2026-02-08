use std::path::{Path, PathBuf};

use anyhow::Result;
use sqlex_analyzer::Analyzer;
use sqlex_common::{
    config::{AnalyzerMode, SqlexConfig},
    ir::{CompilationUnit, QueryDescriptor},
};
use sqlex_database_analyzer::new_database_analyzer;
use sqlex_hybrid_analyzer::HybridAnalyzer;
use sqlex_static_analyzer::StaticAnalyzer;

mod generators;
mod project;

pub use project::Project;

pub struct Compiler {
    config: SqlexConfig,
    config_path: PathBuf,
}

impl Compiler {
    pub fn new(config: SqlexConfig, config_path: &Path) -> Self {
        Self {
            config,
            config_path: config_path.to_path_buf(),
        }
    }

    pub async fn compile(&self) -> Result<()> {
        println!("Compiler: Starting compilation process...");

        // 1. Scan Project
        println!("Step 1: Scanning project files...");
        let project = Project::build(self.config.clone(), &self.config_path).await?;
        println!("  Found {} migration(s)", project.migrations.len());
        println!("  Found {} query(ies)", project.queries.len());

        // 2. Initialize Analyzer
        println!("Step 2: Initializing analyzer...");
        let mut analyzer: Box<dyn Analyzer> = match self.config.analyzer {
            AnalyzerMode::Static => {
                println!("  Using static analyzer");
                Box::new(StaticAnalyzer::new(self.config.dialect))
            },
            AnalyzerMode::Database => {
                println!("  Using database analyzer");
                new_database_analyzer(self.config.dialect).await?
            },
            AnalyzerMode::Hybrid => {
                println!("  Using hybrid analyzer");
                Box::new(HybridAnalyzer::new(self.config.dialect).await?)
            },
        };

        // 3. Analyze Queries
        println!("Step 3: Analyzing queries...");

        // Execute migrations to build schema
        println!("  Executing {} migration(s)...", project.migrations.len());
        for migration in &project.migrations {
            println!(
                "    Executing migration: {} (version {})",
                migration.name, migration.version
            );
            for statement in &migration.statements {
                analyzer
                    .execute(statement)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to execute migration: {}", e))?;
            }
        }

        // Get all tables
        let tables = analyzer
            .get_all_tables()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get tables: {}", e))?;
        println!("  Found {} table(s) in schema", tables.len());

        // Analyze each query
        println!("  Analyzing {} query(ies)...", project.queries.len());
        let mut query_descriptors = Vec::new();
        for query in &project.queries {
            println!("    Analyzing query: {}", query.name);
            let result_set = analyzer
                .analyze(&query.sql)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to analyze query '{}': {}", query.name, e))?;
            query_descriptors.push(QueryDescriptor {
                name: query.name.clone(),
                package_path: query.package_path.clone(),
                sql: query.sql.clone(),
                params: Vec::new(), // TODO: Extract parameters from SQL
                cardinality: result_set.cardinality,
                columns: result_set.columns,
            });
        }

        // Build CompilationUnit
        let compilation_unit = CompilationUnit {
            tables,
            queries: query_descriptors,
        };

        // 4. Initialize & Run Generators
        println!("Step 4: Running generators...");
        for gen_config in &self.config.generators {
            println!(
                "Running generator: {} ({})",
                gen_config.name, gen_config.generator
            );
            let generator =
                generators::get_generator(&gen_config.generator, gen_config.config.clone())
                    .map_err(|e| {
                        anyhow::anyhow!("Failed to create generator '{}': {}", gen_config.name, e)
                    })?;

            generator.generate(&compilation_unit).await?;

            println!("Generator {} finished.", gen_config.name);
        }

        Ok(())
    }
}
