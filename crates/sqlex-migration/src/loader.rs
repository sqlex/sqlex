//! Migration file loader.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use regex::Regex;

use crate::error::MigrationError;
use crate::migration::Migration;

/// Load migrations from a directory.
///
/// Supported filename formats:
/// - `V1__description.sql` (Flyway style)
/// - `001_description.sql` (numeric prefix)
/// - `1_description.sql` (simple numeric)
///
/// Migrations are returned sorted by version number.
pub fn load_migrations(dir: &Path) -> Result<Vec<Migration>, MigrationError> {
    let mut migrations = Vec::new();
    let mut version_map: HashMap<u64, String> = HashMap::new();

    // Patterns for version extraction
    let patterns = [
        // V1__description.sql (Flyway style)
        Regex::new(r"^[Vv](\d+)__(.+)\.sql$").unwrap(),
        // 001_description.sql or 1_description.sql
        Regex::new(r"^(\d+)_(.+)\.sql$").unwrap(),
    ];

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        // Skip non-SQL files
        if !filename.ends_with(".sql") {
            continue;
        }

        // Try to match against patterns
        let mut matched = false;
        for pattern in &patterns {
            if let Some(captures) = pattern.captures(&filename) {
                let version: u64 = captures[1].parse().unwrap();
                let name = captures[2].to_string();

                // Check for duplicate versions
                if let Some(existing) = version_map.get(&version) {
                    return Err(MigrationError::DuplicateVersion {
                        version,
                        first: existing.clone(),
                        second: filename,
                    });
                }

                // Read SQL content
                let sql = fs::read_to_string(&path)?;

                version_map.insert(version, filename.clone());
                migrations.push(Migration {
                    version,
                    name,
                    sql,
                    filename: filename.clone(),
                });

                matched = true;
                break;
            }
        }

        if !matched {
            return Err(MigrationError::InvalidFilename { filename });
        }
    }

    // Sort by version
    migrations.sort();

    Ok(migrations)
}

/// Load migrations from multiple SQL strings (for testing).
pub fn load_migrations_from_strings<'a>(
    sqls: impl IntoIterator<Item = (u64, &'a str, &'a str)>,
) -> Vec<Migration> {
    let mut migrations: Vec<Migration> = sqls
        .into_iter()
        .map(|(version, name, sql)| Migration::new(version, name, sql))
        .collect();
    migrations.sort();
    migrations
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_load_flyway_style() {
        let dir = tempdir().unwrap();

        File::create(dir.path().join("V1__create_users.sql"))
            .unwrap()
            .write_all(b"CREATE TABLE users (id INT);")
            .unwrap();

        File::create(dir.path().join("V2__add_email.sql"))
            .unwrap()
            .write_all(b"ALTER TABLE users ADD COLUMN email VARCHAR(100);")
            .unwrap();

        let migrations = load_migrations(dir.path()).unwrap();
        assert_eq!(migrations.len(), 2);
        assert_eq!(migrations[0].version, 1);
        assert_eq!(migrations[0].name, "create_users");
        assert_eq!(migrations[1].version, 2);
        assert_eq!(migrations[1].name, "add_email");
    }

    #[test]
    fn test_load_numeric_prefix() {
        let dir = tempdir().unwrap();

        File::create(dir.path().join("001_create_users.sql"))
            .unwrap()
            .write_all(b"CREATE TABLE users (id INT);")
            .unwrap();

        let migrations = load_migrations(dir.path()).unwrap();
        assert_eq!(migrations.len(), 1);
        assert_eq!(migrations[0].version, 1);
    }

    #[test]
    fn test_duplicate_version() {
        let dir = tempdir().unwrap();

        File::create(dir.path().join("V1__first.sql"))
            .unwrap()
            .write_all(b"SELECT 1;")
            .unwrap();

        File::create(dir.path().join("001_second.sql"))
            .unwrap()
            .write_all(b"SELECT 2;")
            .unwrap();

        let result = load_migrations(dir.path());
        assert!(matches!(result, Err(MigrationError::DuplicateVersion { .. })));
    }

    #[test]
    fn test_load_from_strings() {
        let migrations = load_migrations_from_strings([
            (2, "second", "SELECT 2;"),
            (1, "first", "SELECT 1;"),
            (3, "third", "SELECT 3;"),
        ]);

        assert_eq!(migrations.len(), 3);
        assert_eq!(migrations[0].version, 1);
        assert_eq!(migrations[1].version, 2);
        assert_eq!(migrations[2].version, 3);
    }
}
