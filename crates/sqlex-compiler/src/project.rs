use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use sqlex_common::config::SqlexConfig;
use sqlparser::{dialect::GenericDialect, parser::Parser};

#[derive(Debug, Clone)]
pub struct Project {
    pub config: SqlexConfig,
    pub config_dir: PathBuf,
    pub migrations: Vec<Migration>,
    pub queries: Vec<Query>,
}

#[derive(Debug, Clone)]
pub struct Migration {
    pub version: u32,
    pub name: String,
    pub statements: Vec<String>,
    pub file_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Query {
    pub name: String,
    pub sql: String,
    pub package_path: Vec<String>,
    pub file_path: PathBuf,
}

impl Project {
    /// Build a project from SqlexConfig and config file path
    pub async fn build(config: SqlexConfig, config_path: &Path) -> Result<Self> {
        let config_dir = config_path
            .parent()
            .ok_or_else(|| anyhow!("Invalid config path: {}", config_path.display()))?
            .to_path_buf();

        let mut project = Self {
            config,
            config_dir,
            migrations: Vec::new(),
            queries: Vec::new(),
        };

        project.scan_migrations().await?;
        project.scan_queries().await?;

        Ok(project)
    }

    /// Scan migrations directory
    async fn scan_migrations(&mut self) -> Result<()> {
        let migrations_dir = self.config_dir.join(&self.config.migrations);

        if !migrations_dir.exists() {
            return Err(anyhow!(
                "Migrations directory does not exist: {}",
                migrations_dir.display()
            ));
        }

        if !migrations_dir.is_dir() {
            return Err(anyhow!(
                "Migrations path is not a directory: {}",
                migrations_dir.display()
            ));
        }

        let mut entries = tokio::fs::read_dir(&migrations_dir)
            .await
            .context("Failed to read migrations directory")?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .context("Failed to read directory entry")?
        {
            let path = entry.path();

            if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("sql") {
                continue;
            }

            let migration = self.parse_migration_file(&path).await?;
            self.migrations.push(migration);
        }

        self.migrations.sort_by_key(|m| m.version);
        self.validate_version_sequence()?;

        Ok(())
    }

    /// Parse a single migration file
    async fn parse_migration_file(&self, path: &Path) -> Result<Migration> {
        let filename = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Invalid filename: {}", path.display()))?;

        let parts: Vec<&str> = filename.splitn(2, '_').collect();
        if parts.len() != 2 {
            return Err(anyhow!(
                "Invalid migration filename format: {}. Expected format: {{version}}_{{name}}.sql",
                path.display()
            ));
        }

        let version_str = parts[0];
        let name = parts[1].to_string();

        Self::validate_identifier(&name, &format!("migration file {}", path.display()))?;

        let version = version_str.parse::<u32>().context(format!(
            "Invalid version number '{}' in file: {}",
            version_str,
            path.display()
        ))?;

        let content = tokio::fs::read_to_string(path)
            .await
            .context(format!("Failed to read migration file: {}", path.display()))?;

        let statements = Self::split_sql_statements(&content);

        Ok(Migration {
            version,
            name,
            statements,
            file_path: path.to_path_buf(),
        })
    }

    /// Split SQL content into individual statements
    fn split_sql_statements(content: &str) -> Vec<String> {
        let dialect = GenericDialect {};
        match Parser::parse_sql(&dialect, content) {
            Ok(statements) => statements.iter().map(|stmt| stmt.to_string()).collect(),
            Err(_) => {
                // Fallback to simple split if parsing fails
                content
                    .split(';')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect()
            },
        }
    }

    /// Validate that migration versions form a valid sequence
    fn validate_version_sequence(&self) -> Result<()> {
        if self.migrations.is_empty() {
            return Ok(());
        }

        if self.migrations[0].version != 0 {
            return Err(anyhow!(
                "First migration version must be 0, found: {}",
                self.migrations[0].version
            ));
        }

        for i in 1..self.migrations.len() {
            let prev = &self.migrations[i - 1];
            let curr = &self.migrations[i];

            if curr.version == prev.version {
                return Err(anyhow!(
                    "Duplicate migration version {}: {} and {}",
                    curr.version,
                    prev.file_path.display(),
                    curr.file_path.display()
                ));
            }

            if curr.version != prev.version + 1 {
                return Err(anyhow!(
                    "Migration version gap detected: expected {}, found {}",
                    prev.version + 1,
                    curr.version
                ));
            }
        }

        Ok(())
    }

    /// Scan all query files (all .sql files except those in migrations directory)
    async fn scan_queries(&mut self) -> Result<()> {
        let migrations_dir = self.config_dir.join(&self.config.migrations);
        self.scan_queries_recursive(&self.config_dir.clone(), &migrations_dir)
            .await?;
        Ok(())
    }

    /// Recursively scan directory for query files
    async fn scan_queries_recursive(
        &mut self,
        current_dir: &Path,
        migrations_dir: &Path,
    ) -> Result<()> {
        let mut entries = tokio::fs::read_dir(current_dir).await.context(format!(
            "Failed to read directory: {}",
            current_dir.display()
        ))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .context("Failed to read directory entry")?
        {
            let path = entry.path();

            // Skip migrations directory
            if path == migrations_dir {
                continue;
            }

            if path.is_dir() {
                Box::pin(self.scan_queries_recursive(&path, migrations_dir)).await?;
                continue;
            }

            if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("sql") {
                continue;
            }

            let file_queries = self.parse_query_file(&path).await?;
            self.queries.extend(file_queries);
        }

        Ok(())
    }

    /// Parse queries from a single SQL file
    async fn parse_query_file(&self, path: &Path) -> Result<Vec<Query>> {
        let file_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Invalid filename: {}", path.display()))?;

        Self::validate_identifier(file_name, &format!("query file {}", path.display()))?;

        // Calculate package path from relative path
        let relative_path = path.strip_prefix(&self.config_dir).context(format!(
            "Failed to compute relative path for {}",
            path.display()
        ))?;

        let package_path: Vec<String> = relative_path
            .parent()
            .unwrap_or(Path::new(""))
            .components()
            .filter_map(|c| c.as_os_str().to_str().map(|s| s.to_string()))
            .collect();

        let content = tokio::fs::read_to_string(path)
            .await
            .context(format!("Failed to read query file: {}", path.display()))?;

        Self::parse_queries_from_content(&content, package_path, path)
    }

    /// Parse queries from content using sqlparser
    fn parse_queries_from_content(
        content: &str,
        package_path: Vec<String>,
        path: &Path,
    ) -> Result<Vec<Query>> {
        let dialect = GenericDialect {};
        let mut queries = Vec::new();

        // First, parse the entire file to get all SQL statements (AST)
        let statements = Parser::parse_sql(&dialect, content)
            .context(format!("Failed to parse SQL file: {}", path.display()))?;

        // Extract query names from comments by scanning the content
        let mut query_names = Vec::new();
        for line in content.lines() {
            if let Some(name_part) = line.trim().strip_prefix("-- name:") {
                let name = name_part.trim().to_string();
                Self::validate_query_name(&name, path)?;
                query_names.push(name);
            }
        }

        // Verify we have matching number of names and statements
        if query_names.len() != statements.len() {
            return Err(anyhow!(
                "Mismatch between query names ({}) and SQL statements ({}) in file: {}",
                query_names.len(),
                statements.len(),
                path.display()
            ));
        }

        // Traverse AST and associate each statement with its query name
        for (name, statement) in query_names.into_iter().zip(statements.iter()) {
            queries.push(Query {
                name,
                sql: statement.to_string(),
                package_path: package_path.clone(),
                file_path: path.to_path_buf(),
            });
        }

        Ok(queries)
    }

    /// Validate identifier naming rules
    fn validate_identifier(name: &str, context: &str) -> Result<()> {
        if name.is_empty() {
            return Err(anyhow!("Empty identifier in {}", context));
        }

        let first_char = name.chars().next().unwrap();
        if !first_char.is_ascii_alphabetic() {
            return Err(anyhow!(
                "Invalid identifier '{}' in {}. Must start with a letter",
                name,
                context
            ));
        }

        for ch in name.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '_' {
                return Err(anyhow!(
                    "Invalid identifier '{}' in {}. Must contain only letters, digits, and underscores",
                    name,
                    context
                ));
            }
        }

        Ok(())
    }

    /// Validate query name follows snake_case convention
    fn validate_query_name(name: &str, path: &Path) -> Result<()> {
        Self::validate_identifier(name, &format!("query name in file {}", path.display()))?;

        for ch in name.chars() {
            if ch.is_ascii_alphabetic() && !ch.is_ascii_lowercase() {
                return Err(anyhow!(
                    "Invalid query name '{}' in file: {}. Query names must be lowercase (snake_case)",
                    name,
                    path.display()
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sqlex_common::{config::AnalyzerMode, dialect::Dialect};

    use super::*;

    fn create_test_config() -> SqlexConfig {
        SqlexConfig {
            name: "test".to_string(),
            dialect: Dialect::SQLite,
            migrations: "migrations".to_string(),
            analyzer: AnalyzerMode::Static,
            generators: vec![],
        }
    }

    #[test]
    fn test_validate_identifier_valid() {
        assert!(Project::validate_identifier("valid_name", "test").is_ok());
        assert!(Project::validate_identifier("ValidName", "test").is_ok());
        assert!(Project::validate_identifier("valid123", "test").is_ok());
        assert!(Project::validate_identifier("a", "test").is_ok());
        assert!(Project::validate_identifier("_valid", "test").is_err()); // Must start with letter
        assert!(Project::validate_identifier("valid_name_123", "test").is_ok());
    }

    #[test]
    fn test_validate_identifier_invalid_empty() {
        let result = Project::validate_identifier("", "test context");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Empty identifier"));
    }

    #[test]
    fn test_validate_identifier_invalid_start() {
        let result = Project::validate_identifier("123invalid", "test context");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Must start with a letter")
        );
    }

    #[test]
    fn test_validate_identifier_invalid_chars() {
        assert!(Project::validate_identifier("invalid-name", "test").is_err());
        assert!(Project::validate_identifier("invalid.name", "test").is_err());
        assert!(Project::validate_identifier("invalid name", "test").is_err());
        assert!(Project::validate_identifier("invalid@name", "test").is_err());
    }

    #[test]
    fn test_validate_query_name_valid() {
        let path = Path::new("test.sql");
        assert!(Project::validate_query_name("valid_query", path).is_ok());
        assert!(Project::validate_query_name("get_user", path).is_ok());
        assert!(Project::validate_query_name("list_all_items", path).is_ok());
        assert!(Project::validate_query_name("a", path).is_ok());
    }

    #[test]
    fn test_validate_query_name_invalid_uppercase() {
        let path = Path::new("test.sql");
        let result = Project::validate_query_name("InvalidQuery", path);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must be lowercase")
        );
    }

    #[test]
    fn test_validate_query_name_invalid_mixed_case() {
        let path = Path::new("test.sql");
        assert!(Project::validate_query_name("getUser", path).is_err());
        assert!(Project::validate_query_name("Get_user", path).is_err());
    }

    #[test]
    fn test_split_sql_statements_single() {
        let sql = "SELECT * FROM users";
        let statements = Project::split_sql_statements(sql);
        assert_eq!(statements.len(), 1);
        assert!(statements[0].contains("SELECT"));
    }

    #[test]
    fn test_split_sql_statements_multiple() {
        let sql = "CREATE TABLE users (id INT); INSERT INTO users VALUES (1);";
        let statements = Project::split_sql_statements(sql);
        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("CREATE TABLE"));
        assert!(statements[1].contains("INSERT INTO"));
    }

    #[test]
    fn test_split_sql_statements_with_whitespace() {
        let sql = "  SELECT * FROM users  ;  \n  INSERT INTO logs VALUES (1)  ;  ";
        let statements = Project::split_sql_statements(sql);
        assert_eq!(statements.len(), 2);
    }

    #[test]
    fn test_split_sql_statements_empty() {
        let sql = "";
        let statements = Project::split_sql_statements(sql);
        assert_eq!(statements.len(), 0);
    }

    #[test]
    fn test_validate_version_sequence_empty() {
        let project = Project {
            config: create_test_config(),
            config_dir: PathBuf::new(),
            migrations: vec![],
            queries: vec![],
        };
        assert!(project.validate_version_sequence().is_ok());
    }

    #[test]
    fn test_validate_version_sequence_valid() {
        let project = Project {
            config: create_test_config(),
            config_dir: PathBuf::new(),
            migrations: vec![
                Migration {
                    version: 0,
                    name: "init".to_string(),
                    statements: vec![],
                    file_path: PathBuf::from("0_init.sql"),
                },
                Migration {
                    version: 1,
                    name: "add_users".to_string(),
                    statements: vec![],
                    file_path: PathBuf::from("1_add_users.sql"),
                },
                Migration {
                    version: 2,
                    name: "add_posts".to_string(),
                    statements: vec![],
                    file_path: PathBuf::from("2_add_posts.sql"),
                },
            ],
            queries: vec![],
        };
        assert!(project.validate_version_sequence().is_ok());
    }

    #[test]
    fn test_validate_version_sequence_invalid_start() {
        let project = Project {
            config: create_test_config(),
            config_dir: PathBuf::new(),
            migrations: vec![Migration {
                version: 1,
                name: "init".to_string(),
                statements: vec![],
                file_path: PathBuf::from("1_init.sql"),
            }],
            queries: vec![],
        };
        let result = project.validate_version_sequence();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("First migration version must be 0")
        );
    }

    #[test]
    fn test_validate_version_sequence_duplicate() {
        let project = Project {
            config: create_test_config(),
            config_dir: PathBuf::new(),
            migrations: vec![
                Migration {
                    version: 0,
                    name: "init".to_string(),
                    statements: vec![],
                    file_path: PathBuf::from("0_init.sql"),
                },
                Migration {
                    version: 0,
                    name: "duplicate".to_string(),
                    statements: vec![],
                    file_path: PathBuf::from("0_duplicate.sql"),
                },
            ],
            queries: vec![],
        };
        let result = project.validate_version_sequence();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Duplicate migration version")
        );
    }

    #[test]
    fn test_validate_version_sequence_gap() {
        let project = Project {
            config: create_test_config(),
            config_dir: PathBuf::new(),
            migrations: vec![
                Migration {
                    version: 0,
                    name: "init".to_string(),
                    statements: vec![],
                    file_path: PathBuf::from("0_init.sql"),
                },
                Migration {
                    version: 2,
                    name: "skip".to_string(),
                    statements: vec![],
                    file_path: PathBuf::from("2_skip.sql"),
                },
            ],
            queries: vec![],
        };
        let result = project.validate_version_sequence();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Migration version gap detected")
        );
    }

    #[test]
    fn test_parse_queries_from_content_single() {
        let content = "-- name: get_user\nSELECT * FROM users WHERE id = 1";
        let path = Path::new("test.sql");
        let package_path = vec![];

        let result = Project::parse_queries_from_content(content, package_path.clone(), path);
        assert!(result.is_ok());

        let queries = result.unwrap();
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].name, "get_user");
        assert!(queries[0].sql.contains("SELECT"));
        assert_eq!(queries[0].package_path, package_path);
    }

    #[test]
    fn test_parse_queries_from_content_multiple() {
        let content = r#"
-- name: get_user
SELECT * FROM users WHERE id = 1;

-- name: list_users
SELECT * FROM users;
"#;
        let path = Path::new("test.sql");
        let package_path = vec!["queries".to_string()];

        let result = Project::parse_queries_from_content(content, package_path.clone(), path);
        assert!(result.is_ok());

        let queries = result.unwrap();
        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0].name, "get_user");
        assert_eq!(queries[1].name, "list_users");
        assert_eq!(queries[0].package_path, package_path);
        assert_eq!(queries[1].package_path, package_path);
    }

    #[test]
    fn test_parse_queries_from_content_mismatch() {
        let content = "-- name: get_user\nSELECT * FROM users; SELECT * FROM posts;";
        let path = Path::new("test.sql");
        let package_path = vec![];

        let result = Project::parse_queries_from_content(content, package_path, path);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Mismatch between query names")
        );
    }

    #[test]
    fn test_parse_queries_from_content_invalid_name() {
        let content = "-- name: InvalidName\nSELECT * FROM users";
        let path = Path::new("test.sql");
        let package_path = vec![];

        let result = Project::parse_queries_from_content(content, package_path, path);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must be lowercase")
        );
    }
}
