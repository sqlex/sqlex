use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::{LevelFilter, error, info};
use notify::{RecursiveMode, Watcher};
use sqlex_common::{
    config::{AnalyzerMode, GeneratorConfig, SqlexConfig},
    dialect::Dialect,
};
use sqlex_compiler::Compiler;
use tokio::{fs, time::Instant};

#[derive(Parser)]
#[command(name = "sqlex")]
#[command(about = "SQL Code Generator & Analyzer", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new sqlex project
    Init {
        /// Project name (defaults to current directory name if not provided)
        name: Option<String>,
    },
    /// Add script generator to existing project
    Script {
        /// Path to configuration file or directory (searches upward from current directory if not provided)
        config: Option<String>,
    },
    /// Generate code
    Generate {
        /// Path to configuration file or directory (searches upward from current directory if not provided)
        config: Option<String>,
    },
    /// Watch for changes and generate code
    Watch {
        /// Path to configuration file or directory (searches upward from current directory if not provided)
        config: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Warn)
        .with_module_level("sqlex", LevelFilter::Info)
        .with_module_level("sqlex_cli", LevelFilter::Info)
        .with_module_level("sqlex_compiler", LevelFilter::Info)
        .with_module_level("sqlex_analyzer", LevelFilter::Info)
        .with_module_level("sqlex_generator", LevelFilter::Info)
        .with_module_level("sqlex_static_analyzer", LevelFilter::Info)
        .with_module_level("sqlex_database_analyzer", LevelFilter::Info)
        .with_module_level("sqlex_hybrid_analyzer", LevelFilter::Info)
        .with_module_level("sqlex_generator_rust", LevelFilter::Info)
        .with_module_level("sqlex_generator_go", LevelFilter::Info)
        .with_module_level("sqlex_generator_java", LevelFilter::Info)
        .with_module_level("sqlex_generator_debug", LevelFilter::Info)
        .with_module_level("sqlex_common", LevelFilter::Info)
        .init()
        .ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => run_init(name).await?,
        Commands::Script { config } => run_script(config).await?,
        Commands::Generate { config } => run_generate(config).await?,
        Commands::Watch { config } => run_watch(config).await?,
    }

    Ok(())
}

async fn run_init(name: Option<String>) -> Result<()> {
    let (root, project_name) = if let Some(name) = name {
        // Create new directory with given name
        (PathBuf::from(&name), name)
    } else {
        // Use current directory
        let current_dir = std::env::current_dir().context("Failed to get current directory")?;
        let dir_name = current_dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Failed to get current directory name"))?
            .to_string();
        (current_dir, dir_name)
    };

    if root.exists() {
        if !root.is_dir() {
            anyhow::bail!("path {} exists but is not a directory", root.display());
        }
    } else {
        fs::create_dir_all(&root)
            .await
            .context("Failed to create project directory")?;
    }

    let config_path = root.join("sqlex.yaml");
    if config_path.exists() {
        anyhow::bail!("config file already exists at {:?}", config_path);
    } else {
        let config = SqlexConfig {
            name: project_name.clone(),
            dialect: Dialect::Postgres,
            migrations: "migrations".to_string(),
            analyzer: AnalyzerMode::Hybrid,
            generators: vec![GeneratorConfig {
                name: "debug_output".to_string(),
                generator: "debug".to_string(),
                output: PathBuf::from("generated"),
                config: serde_json::json!({
                    "format": "text"
                }),
            }],
        };
        let content = serde_yaml::to_string(&config)?;
        fs::write(&config_path, content)
            .await
            .context("Failed to write sqlex.yaml")?;
        info!("created {:?}", config_path);
    }

    let migrations_dir = root.join("migrations");
    if !migrations_dir.exists() {
        fs::create_dir_all(&migrations_dir)
            .await
            .context("Failed to create migrations directory")?;
        info!("created {:?}", migrations_dir);
    }

    info!("initialized sqlex project: {}", project_name);
    Ok(())
}

async fn run_script(config_path: Option<String>) -> Result<()> {
    let config_file = find_config_file(config_path).await?;
    let project_root = config_file
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Failed to get project root"))?;

    // Read existing config
    let config_content = fs::read_to_string(&config_file)
        .await
        .context("Failed to read sqlex.yaml")?;
    let mut config: SqlexConfig = serde_yaml::from_str(&config_content)?;

    // Check if script generator already exists
    let has_script_generator = config.generators.iter().any(|g| g.generator == "script");

    if has_script_generator {
        anyhow::bail!("Script generator already exists in configuration");
    }

    // Add script generator configuration
    config.generators.push(GeneratorConfig {
        name: "script_generator".to_string(),
        generator: "script".to_string(),
        output: PathBuf::from("generated"),
        config: serde_json::json!({
            "script": "scripts/generate.ts"
        }),
    });

    // Save updated config
    let updated_content = serde_yaml::to_string(&config)?;
    fs::write(&config_file, updated_content)
        .await
        .context("Failed to write updated sqlex.yaml")?;
    info!("updated {:?}", config_file);

    // Create scripts directory
    let scripts_dir = project_root.join("scripts");
    if !scripts_dir.exists() {
        fs::create_dir_all(&scripts_dir)
            .await
            .context("Failed to create scripts directory")?;
        info!("created {:?}", scripts_dir);
    }

    // Generate example script
    let example_script_path = scripts_dir.join("generate.ts");
    if !example_script_path.exists() {
        let example_content = sqlex_generator_script::generate_example_script();
        fs::write(&example_script_path, example_content)
            .await
            .context("Failed to write example script")?;
        info!("created {:?}", example_script_path);
    } else {
        info!("skipped {:?} (already exists)", example_script_path);
    }

    // Generate type definitions
    let type_defs_path = scripts_dir.join("sqlex-types.d.ts");
    if !type_defs_path.exists() {
        let type_defs_content = sqlex_generator_script::generate_type_definitions();
        fs::write(&type_defs_path, type_defs_content)
            .await
            .context("Failed to write type definitions")?;
        info!("created {:?}", type_defs_path);
    } else {
        info!("skipped {:?} (already exists)", type_defs_path);
    }

    info!("script generator added successfully!");
    Ok(())
}

async fn run_generate(config_path: Option<String>) -> Result<()> {
    let config_file = find_config_file(config_path).await?;
    let mut compiler = Compiler::new(&config_file).await?;
    compiler.compile().await?;
    info!("generated successfully!");
    Ok(())
}

async fn run_watch(config_path: Option<String>) -> Result<()> {
    let config_file = find_config_file(config_path.clone()).await?;
    let project_root = config_file.parent().unwrap_or_else(|| Path::new("."));

    // Create compiler instance (will be reused for incremental compilation)
    let mut compiler = Compiler::new(&config_file).await?;

    // Initial run
    if let Err(e) = compiler.compile().await {
        error!("initial generation failed: {:#}", e);
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;

    // Watch entire project root directory recursively
    watcher.watch(project_root, RecursiveMode::Recursive)?;
    info!("watching project directory: {:?}", project_root);

    // Debounce logic
    let debounce_duration = Duration::from_secs(1);
    let mut next_run: Option<Instant> = None;

    loop {
        // Use a far future deadline when next_run is None to effectively disable the timer branch
        let deadline = next_run.unwrap_or(Instant::now() + Duration::from_secs(86400 * 365));

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("received interrupt signal, shutting down...");
                break;
            }
            _ = tokio::time::sleep_until(deadline) => {
                // Timer expired, execute the compilation only if next_run was Some
                if next_run.is_some() {
                    info!("file change detected. regenerating...");

                    // Reuse compiler for incremental compilation
                    if let Err(e) = compiler.compile().await {
                        error!("generation failed: {:#}", e);
                    }

                    next_run = None;
                }
            }
            event_res = rx.recv() => {
                match event_res {
                    Some(Ok(_)) => {
                        // Reset the debounce timer on each file change event
                        next_run = Some(Instant::now() + debounce_duration);
                    },
                    Some(Err(e)) => error!("watch error: {:?}", e),
                    None => {
                        error!("watch channel closed");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn find_config_file(path_str: Option<String>) -> Result<PathBuf> {
    let config_path = if let Some(path_str) = path_str {
        // User provided a path
        let path = Path::new(&path_str);
        if path.is_dir() {
            path.join("sqlex.yaml")
        } else {
            path.to_path_buf()
        }
    } else {
        // Search upward from current directory
        let mut current = std::env::current_dir().context("Failed to get current directory")?;

        loop {
            let config_path = current.join("sqlex.yaml");
            if config_path.exists() {
                break config_path;
            }

            if !current.pop() {
                anyhow::bail!("sqlex.yaml not found in current directory or any parent directory");
            }
        }
    };

    if !config_path.exists() {
        anyhow::bail!("Config file not found at: {:?}", config_path);
    }

    Ok(config_path)
}
