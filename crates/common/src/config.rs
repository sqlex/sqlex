use serde::{Deserialize, Serialize};

use crate::Dialect;

#[derive(Debug, Serialize, Deserialize)]
pub struct SqlexConfig {
    pub name: String,
    pub dialect: Dialect, // postgres, mysql, sqlite
    #[serde(default = "default_migrations_dir")]
    pub migrations: String,
    #[serde(default)]
    pub analyzer: AnalyzerMode,
    #[serde(default)]
    pub generators: Vec<GeneratorConfig>,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum AnalyzerMode {
    Static,
    #[default]
    Database,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneratorConfig {
    pub name: String,
    pub generator: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

fn default_migrations_dir() -> String {
    "migrations".to_string()
}
