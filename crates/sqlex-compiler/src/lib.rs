use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use anyhow::Result;
use log::{debug, info};
use sqlex_analyzer::Analyzer;
use sqlex_common::{
    config::{AnalyzerMode, SqlexConfig},
    ir::{CompilationUnit, QueryDescriptor},
};
use sqlex_database_analyzer::new_database_analyzer;
use sqlex_generator::Generator;
use sqlex_hybrid_analyzer::HybridAnalyzer;
use sqlex_static_analyzer::StaticAnalyzer;

mod generators;
mod project;

pub use project::Project;

pub struct Compiler {
    config: SqlexConfig,
    config_path: PathBuf,
    state: Option<CompilationState>,
}

struct CompilationState {
    migration_hashes: HashMap<PathBuf, u64>,
    query_file_hashes: HashMap<PathBuf, u64>, // file_path -> file_hash
    config_hash: u64,
    analyzer: Box<dyn Analyzer>,
    generators: Vec<Box<dyn Generator>>,
    compilation_unit: CompilationUnit,
    project: Project,
}

enum ChangeType {
    FirstRun,
    ConfigChanged,
    MigrationsChanged,
    QueriesOnlyChanged {
        added: Vec<usize>,
        modified: Vec<usize>,
        deleted: Vec<(Vec<String>, String)>, // (package_path, name)
    },
    NoChange,
}

impl Compiler {
    pub fn new(config: SqlexConfig, config_path: &Path) -> Self {
        Self {
            config,
            config_path: config_path.to_path_buf(),
            state: None,
        }
    }

    async fn compute_file_hash(path: &Path) -> Result<u64> {
        let content = tokio::fs::read(path).await?;
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        Ok(hasher.finish())
    }

    fn compute_config_hash(config: &SqlexConfig) -> u64 {
        let mut hasher = DefaultHasher::new();
        if let Ok(json) = serde_json::to_string(config) {
            json.hash(&mut hasher);
        }
        hasher.finish()
    }

    async fn detect_changes(&self, new_project: &Project) -> Result<ChangeType> {
        let Some(state) = &self.state else {
            return Ok(ChangeType::FirstRun);
        };

        // Check config changes
        let new_config_hash = Self::compute_config_hash(&self.config);
        if new_config_hash != state.config_hash {
            info!("detected config change");
            return Ok(ChangeType::ConfigChanged);
        }

        // Check migrations changes
        if new_project.migrations.len() != state.project.migrations.len() {
            info!("detected migrations count change");
            return Ok(ChangeType::MigrationsChanged);
        }

        for (new_mig, old_mig) in new_project.migrations.iter().zip(&state.project.migrations) {
            if new_mig.file_path != old_mig.file_path {
                info!("detected migration file path change");
                return Ok(ChangeType::MigrationsChanged);
            }

            let new_hash = Self::compute_file_hash(&new_mig.file_path).await?;
            let old_hash = state.migration_hashes.get(&new_mig.file_path);

            if old_hash.is_none() || *old_hash.unwrap() != new_hash {
                info!(
                    "detected migration file content change: {:?}",
                    new_mig.file_path
                );
                return Ok(ChangeType::MigrationsChanged);
            }
        }

        // Check queries changes
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();

        // Build old query files set
        let old_query_files: std::collections::HashSet<_> =
            state.project.queries.iter().map(|q| &q.file_path).collect();

        // Collect unique file paths from new project and compute hashes once per file
        let mut new_file_hashes: HashMap<PathBuf, u64> = HashMap::new();
        let mut seen_files = std::collections::HashSet::new();

        for new_query in &new_project.queries {
            if seen_files.insert(new_query.file_path.clone()) {
                let new_hash = Self::compute_file_hash(&new_query.file_path).await?;
                new_file_hashes.insert(new_query.file_path.clone(), new_hash);
            }
        }

        // Check for added and modified queries (file level)
        for (new_idx, new_query) in new_project.queries.iter().enumerate() {
            if old_query_files.contains(&new_query.file_path) {
                // File exists in old project, check if modified
                let new_hash = new_file_hashes.get(&new_query.file_path).unwrap();
                let old_hash = state.query_file_hashes.get(&new_query.file_path);

                if old_hash.is_none() || old_hash.unwrap() != new_hash {
                    modified.push(new_idx);
                }
            } else {
                // New file
                added.push(new_idx);
            }
        }

        // Check for deleted queries using (package_path, name) as unique identifier
        let new_queries_set: std::collections::HashSet<_> = new_project
            .queries
            .iter()
            .map(|q| (q.package_path.clone(), q.name.clone()))
            .collect();

        for old_query in &state.project.queries {
            let query_key = (old_query.package_path.clone(), old_query.name.clone());
            if !new_queries_set.contains(&query_key) {
                deleted.push(query_key);
            }
        }

        if !added.is_empty() || !modified.is_empty() || !deleted.is_empty() {
            info!(
                "detected query changes: {} added, {} modified, {} deleted",
                added.len(),
                modified.len(),
                deleted.len()
            );
            return Ok(ChangeType::QueriesOnlyChanged {
                added,
                modified,
                deleted,
            });
        }

        Ok(ChangeType::NoChange)
    }

    async fn full_compile(&mut self, project: Project) -> Result<()> {
        info!("performing full compilation");

        // Initialize Analyzer
        info!("initializing analyzer...");
        let mut analyzer: Box<dyn Analyzer> = match self.config.analyzer {
            AnalyzerMode::Static => {
                info!("  using static analyzer");
                Box::new(StaticAnalyzer::new(self.config.dialect))
            },
            AnalyzerMode::Database => {
                info!("  using database analyzer");
                new_database_analyzer(self.config.dialect).await?
            },
            AnalyzerMode::Hybrid => {
                info!("  using hybrid analyzer");
                Box::new(HybridAnalyzer::new(self.config.dialect).await?)
            },
        };

        // Execute migrations to build schema
        info!("executing {} migration(s)...", project.migrations.len());
        for migration in &project.migrations {
            debug!(
                "  executing migration: {} (version {})",
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
        info!("found {} table(s) in schema", tables.len());

        // Analyze each query
        info!("analyzing {} query(ies)...", project.queries.len());
        let mut query_descriptors = Vec::new();
        for query in &project.queries {
            debug!("  analyzing query: {}", query.name);
            let result_set = analyzer
                .analyze(&query.sql)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to analyze query '{}': {}", query.name, e))?;
            query_descriptors.push(QueryDescriptor {
                name: query.name.clone(),
                package_path: query.package_path.clone(),
                sql: query.sql.clone(),
                params: Vec::new(),
                cardinality: result_set.cardinality,
                columns: result_set.columns,
            });
        }

        // Build CompilationUnit
        let compilation_unit = CompilationUnit {
            tables,
            queries: query_descriptors,
        };

        // Compute migration file hashes
        let mut migration_hashes = HashMap::new();
        for migration in &project.migrations {
            let hash = Self::compute_file_hash(&migration.file_path).await?;
            migration_hashes.insert(migration.file_path.clone(), hash);
        }

        // Compute query file hashes (once per file)
        let mut query_file_hashes = HashMap::new();
        let mut seen_files = std::collections::HashSet::new();
        for query in &project.queries {
            if seen_files.insert(query.file_path.clone()) {
                let hash = Self::compute_file_hash(&query.file_path).await?;
                query_file_hashes.insert(query.file_path.clone(), hash);
            }
        }

        // Initialize generators
        info!("initializing generators...");
        let mut generators = Vec::new();
        for gen_config in &self.config.generators {
            info!("  initializing generator: {}", gen_config.name);
            let generator = generators::get_generator(
                &gen_config.generator,
                &gen_config.output,
                gen_config.config.clone(),
            )
            .map_err(|e| {
                anyhow::anyhow!("Failed to create generator '{}': {}", gen_config.name, e)
            })?;
            generators.push(generator);
        }

        // Save state
        self.state = Some(CompilationState {
            migration_hashes,
            query_file_hashes,
            config_hash: Self::compute_config_hash(&self.config),
            analyzer,
            generators,
            compilation_unit,
            project,
        });

        Ok(())
    }

    async fn incremental_compile(
        &mut self,
        project: Project,
        added: Vec<usize>,
        modified: Vec<usize>,
        deleted: Vec<(Vec<String>, String)>,
    ) -> Result<()> {
        info!("performing incremental compilation");

        let state = self
            .state
            .as_mut()
            .expect("state should exist for incremental compile");

        // Reuse existing analyzer and tables
        info!("reusing analyzer and schema");
        let analyzer = &state.analyzer;

        // Analyze changed queries
        let changed_count = added.len() + modified.len();
        info!("analyzing {} changed query(ies)...", changed_count);

        let mut new_query_descriptors = Vec::new();

        for &idx in added.iter().chain(modified.iter()) {
            let query = &project.queries[idx];
            debug!("  analyzing query: {}", query.name);
            let result_set = analyzer
                .analyze(&query.sql)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to analyze query '{}': {}", query.name, e))?;
            new_query_descriptors.push((
                idx,
                QueryDescriptor {
                    name: query.name.clone(),
                    package_path: query.package_path.clone(),
                    sql: query.sql.clone(),
                    params: Vec::new(),
                    cardinality: result_set.cardinality,
                    columns: result_set.columns,
                },
            ));
        }

        // Update CompilationUnit queries
        // Remove deleted queries
        state.compilation_unit.queries.retain(|q| {
            !deleted
                .iter()
                .any(|(package_path, name)| q.package_path == *package_path && q.name == *name)
        });

        // Update modified queries and add new queries
        let mut updated_files = std::collections::HashSet::new();

        for (idx, new_descriptor) in new_query_descriptors {
            let query = &project.queries[idx];

            // Check if this is a modification using (package_path, name) as unique identifier
            if let Some(existing) = state.compilation_unit.queries.iter_mut().find(|q| {
                q.package_path == new_descriptor.package_path && q.name == new_descriptor.name
            }) {
                *existing = new_descriptor;
            } else {
                // New query
                state.compilation_unit.queries.push(new_descriptor);
            }

            // Track files that need hash update
            updated_files.insert(query.file_path.clone());
        }

        // Update file hashes (once per file)
        for file_path in updated_files {
            let hash = Self::compute_file_hash(&file_path).await?;
            state.query_file_hashes.insert(file_path, hash);
        }

        // Update project reference
        state.project = project;

        Ok(())
    }

    async fn run_generators(&self) -> Result<()> {
        let state = self
            .state
            .as_ref()
            .expect("state should exist when running generators");

        info!("running generators...");
        for (generator, gen_config) in state.generators.iter().zip(&self.config.generators) {
            info!(
                "running generator: {} ({})",
                gen_config.name, gen_config.generator
            );

            generator.generate(&state.compilation_unit).await?;

            info!("generator {} finished.", gen_config.name);
        }

        Ok(())
    }

    pub async fn compile(&mut self) -> Result<()> {
        info!("compiler: starting compilation process...");

        // Scan project files
        info!("scanning project files...");
        let project = Project::build(self.config.clone(), &self.config_path).await?;
        info!("  found {} migration(s)", project.migrations.len());
        info!("  found {} query(ies)", project.queries.len());

        // Detect changes
        let change_type = self.detect_changes(&project).await?;

        match change_type {
            ChangeType::FirstRun | ChangeType::ConfigChanged | ChangeType::MigrationsChanged => {
                // Full compilation
                self.full_compile(project).await?;
            },
            ChangeType::QueriesOnlyChanged {
                added,
                modified,
                deleted,
            } => {
                // Incremental compilation
                self.incremental_compile(project, added, modified, deleted)
                    .await?;
            },
            ChangeType::NoChange => {
                info!("no changes detected, skipping compilation");
                return Ok(());
            },
        }

        // Run generators
        self.run_generators().await?;

        info!("compilation completed successfully");
        Ok(())
    }
}
