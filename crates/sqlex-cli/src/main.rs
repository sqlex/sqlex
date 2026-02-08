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
                name: "rust_entities".to_string(),
                generator: "rust".to_string(),
                output: PathBuf::from("src/entities"),
                config: serde_json::json!({
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

    info!("initialized sqlex project: {}", project_name);
    Ok(())
}

async fn run_generate(config_path: Option<String>) -> Result<()> {
    let (config_file, config) = load_config(config_path).await?;
    let mut compiler = Compiler::new(config, &config_file);
    compiler.compile().await?;
    info!("generated successfully!");
    Ok(())
}

async fn run_watch(config_path: Option<String>) -> Result<()> {
    let (config_file, config) = load_config(config_path.clone()).await?;
    let project_root = config_file.parent().unwrap_or_else(|| Path::new("."));

    // Create compiler instance (will be reused for incremental compilation)
    let mut compiler = Compiler::new(config, &config_file);

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

async fn load_config(path_str: Option<String>) -> Result<(PathBuf, SqlexConfig)> {
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

    let content = fs::read_to_string(&config_path)
        .await
        .context(format!("Failed to read config file: {:?}", config_path))?;

    let mut config: SqlexConfig =
        serde_yaml::from_str(&content).context("Failed to parse config file")?;

    // Resolve relative paths for generator outputs
    let config_dir = config_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Failed to get config directory"))?;

    for generator in &mut config.generators {
        if generator.output.is_relative() {
            generator.output = config_dir.join(&generator.output);
        }
    }

    Ok((config_path, config))
}
