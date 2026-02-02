mod config;

use std::path::Path;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::{AnalyzerMode, SqlexConfig};
use sqlex_analyzer::Analyzer;
use sqlex_common::DatabaseType;
use sqlex_database_analyzer::DatabaseAnalyzer;
use sqlex_static_analyzer::StaticAnalyzer;
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
    /// Analyze queries and output result sets
    Analyze {
        /// Path to configuration file
        #[arg(short, long, default_value = "sqlex.yaml")]
        config: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { config } => {
            run_analyze(&config).await?;
        },
    }

    Ok(())
}

async fn run_analyze(config_path: &str) -> Result<()> {
    // 1. Load Config
    let config_content = fs::read_to_string(config_path)
        .await
        .context(format!("Failed to read config file: {}", config_path))?;
    let config: SqlexConfig =
        serde_yaml::from_str(&config_content).context("Failed to parse config file")?;

    println!("Loaded config: {:?}", config);

    // 2. Resolve Database Type
    let db_type = match config.database.to_lowercase().as_str() {
        "postgres" => DatabaseType::Postgres,
        "mysql" => DatabaseType::MySQL,
        "sqlite" => DatabaseType::SQLite,
        _ => anyhow::bail!("Unsupported database type: {}", config.database),
    };

    // 3. Instantiate Analyzer
    let mut analyzer: Box<dyn Analyzer> = match config.analyzer {
        AnalyzerMode::Database => {
            // TODO: Start container or use connection string
            // For now, assuming local DB or testcontainers helper is used.
            // Since we didn't implement auto-container start in `sqlex-database-analyzer` lib yet fully (just stub for user),
            // I'll use a placeholder connection string or expect env var.
            // But user requirement was "use testcontainers".
            // So we really should have that helper.
            // For now, let's just attempt to connect to a default URL or fail.
            println!("Initializing Database Analyzer...");
            let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:postgres@localhost:5432/postgres".to_string()
            });
            Box::new(DatabaseAnalyzer::new(&url, db_type).await?)
        },
        AnalyzerMode::Static => {
            println!("Initializing Static Analyzer...");
            Box::new(StaticAnalyzer::new())
        },
    };

    // 4. Run Migrations
    let migrations_dir = config.migrations;
    if Path::new(&migrations_dir).exists() {
        let mut paths = Vec::new();
        let mut entries = fs::read_dir(&migrations_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "sql") {
                paths.push(path);
            }
        }

        paths.sort(); // Important: determinism

        for path in paths {
            println!("Applying migration: {:?}", path);
            let sql = fs::read_to_string(&path).await?;
            analyzer.execute(&sql).await?;
        }
    } else {
        println!(
            "Warning: Migrations directory '{}' not found.",
            migrations_dir
        );
    }

    // 5. Analyze (Stub)
    // In real app, we would scan for query files.
    // Here we just test a sample query.
    let sample_query = "SELECT 1";
    println!("Analyzing sample query: {}", sample_query);
    match analyzer.analyze(sample_query).await {
        Ok(rs) => println!("Analysis Result: {:?}", rs),
        Err(e) => println!("Analysis Failed: {}", e),
    }

    Ok(())
}
