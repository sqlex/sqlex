//! Migration file loader and executor for sqlex.
//!
//! This crate provides functionality to:
//! - Load migration files from a directory
//! - Parse version numbers from filenames
//! - Apply migrations to build a schema

mod error;
mod loader;
mod migration;

pub use error::MigrationError;
pub use loader::load_migrations;
pub use migration::Migration;

use sqlex_schema::SchemaRegistry;
use sqlex_types::Dialect;

/// Apply migrations to build a schema registry.
///
/// # Arguments
/// * `migrations` - Sorted list of migrations to apply
/// * `dialect` - SQL dialect
///
/// # Returns
/// A schema registry with all migrations applied
pub fn apply_migrations(
    migrations: &[Migration],
    dialect: Dialect,
) -> Result<SchemaRegistry, MigrationError> {
    let mut registry = SchemaRegistry::new(dialect);

    for migration in migrations {
        registry
            .apply_sql(&migration.sql)
            .map_err(|e| MigrationError::Apply {
                version: migration.version,
                name: migration.name.clone(),
                reason: e.to_string(),
            })?;
    }

    Ok(registry)
}
