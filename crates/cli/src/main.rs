use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use sqlex_common::SqlexConfig;
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
            run_compiler(&config).await?;
        },
    }

    Ok(())
}

async fn run_compiler(config_path: &str) -> Result<()> {
    // 1. Load Config
    let config_content = fs::read_to_string(config_path)
        .await
        .context(format!("Failed to read config file: {}", config_path))?;
    let config: SqlexConfig =
        serde_yaml::from_str(&config_content).context("Failed to parse config file")?;

    println!("Loaded config: {:?}", config);

    // 2. Initialize and Run Compiler
    let compiler = Compiler::new(config);
    compiler.compile().await?;

    Ok(())
}
