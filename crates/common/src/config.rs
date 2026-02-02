use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SqlexConfig {
    pub database: String, // postgres, mysql, sqlite
    #[serde(default)]
    pub analyzer: AnalyzerMode,
    #[serde(default = "default_migrations_dir")]
    pub migrations: String,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum AnalyzerMode {
    Static,
    #[default]
    Database,
}

fn default_migrations_dir() -> String {
    "migrations".to_string()
}
