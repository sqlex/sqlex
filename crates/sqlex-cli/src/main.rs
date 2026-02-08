use std::{
    path::{Path, PathBuf},
    sync::mpsc::channel,
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
use tokio::fs;

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
        /// Project name
        name: String,
    },
    /// Generate code
    Generate {
        /// Path to configuration file or directory
        #[arg(default_value = ".")]
        config: String,
    },
    /// Watch for changes and generate code
    Watch {
        /// Path to configuration file or directory
        #[arg(default_value = ".")]
        config: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Warn)
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
        Commands::Generate { config } => run_generate(config).await?,
        Commands::Watch { config } => run_watch(config).await?,
    }

    Ok(())
}

async fn run_init(name: String) -> Result<()> {
    let root = Path::new(&name);
    if root.exists() {
        if !root.is_dir() {
            anyhow::bail!("path {} exists but is not a directory", name);
        }
        // If it exists, we check if it's empty or assume user wants to init here.
        // For safety, we just allow it.
    } else {
        fs::create_dir_all(root)
            .await
            .context("Failed to create project directory")?;
    }

    let config_path = root.join("sqlex.yaml");
    if config_path.exists() {
        info!("config file already exists at {:?}", config_path);
    } else {
        let config = SqlexConfig {
            name: name.clone(),
            dialect: Dialect::Postgres,
            migrations: "migrations".to_string(),
            analyzer: AnalyzerMode::Hybrid,
            generators: vec![GeneratorConfig {
                name: "rust_entities".to_string(),
                generator: "rust".to_string(),
                config: serde_json::json!({
                    "output_dir": "src/entities",
                    "orm_mode": "sqlx"
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

    info!("initialized sqlex project: {}", name);
    Ok(())
}

async fn run_generate(config_path: String) -> Result<()> {
    let (config_file, config) = load_config(&config_path).await?;
    run_compiler(config, &config_file).await?;
    info!("generated successfully!");
    Ok(())
}

async fn run_watch(config_path: String) -> Result<()> {
    // Initial run
    if let Err(e) = run_generate(config_path.clone()).await {
        error!("initial generation failed: {:#}", e);
    }

    let (config_file, _config) = load_config(&config_path).await?;
    let project_root = config_file.parent().unwrap_or_else(|| Path::new("."));

    let (tx, rx) = channel();
    let mut watcher = notify::recommended_watcher(tx)?;

    // Watch entire project root directory recursively
    watcher.watch(project_root, RecursiveMode::Recursive)?;
    info!("watching project directory: {:?}", project_root);

    // Debounce logic
    let debounce_duration = Duration::from_secs(2);
    let mut last_processed = std::time::Instant::now();

    loop {
        match rx.recv() {
            Ok(event_res) => {
                match event_res {
                    Ok(_) => {
                        let now = std::time::Instant::now();
                        if now.duration_since(last_processed) < debounce_duration {
                            continue;
                        }

                        // Small delay to let FS settle and accumulate more events if any
                        tokio::time::sleep(Duration::from_millis(100)).await;

                        info!("file change detected. regenerating...");

                        if let Err(e) = run_generate(config_path.clone()).await {
                            error!("generation failed: {:#}", e);
                        }
                        last_processed = std::time::Instant::now();
                    },
                    Err(e) => error!("watch error: {:?}", e),
                }
            },
            Err(e) => {
                error!("watch channel error: {:?}", e);
                break;
            },
        }
    }

    Ok(())
}

async fn run_compiler(config: SqlexConfig, config_path: &Path) -> Result<()> {
    let compiler = Compiler::new(config, config_path);
    compiler.compile().await?;
    Ok(())
}

async fn load_config(path_str: &str) -> Result<(PathBuf, SqlexConfig)> {
    let path = Path::new(path_str);
    let config_path = if path.is_dir() {
        path.join("sqlex.yaml")
    } else {
        path.to_path_buf()
    };

    if !config_path.exists() {
        anyhow::bail!("Config file not found at: {:?}", config_path);
    }

    let content = fs::read_to_string(&config_path)
        .await
        .context(format!("Failed to read config file: {:?}", config_path))?;

    let config: SqlexConfig =
        serde_yaml::from_str(&content).context("Failed to parse config file")?;

    Ok((config_path, config))
}
