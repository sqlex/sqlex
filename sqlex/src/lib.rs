//! # sqlex
//!
//! A database schema analyzer that can:
//! 1. Parse SQL migration files to infer the final schema
//! 2. Analyze SQL queries to determine result set metadata (column names, types, nullability)
//!
//! ## Supported Databases
//! - PostgreSQL
//! - MySQL
//! - SQLite
//!
//! ## Example
//!
//! ```rust,no_run
//! use std::path::Path;
//!
//! use sqlex::{Dialect, QueryAnalyzer, apply_migrations, load_migrations};
//!
//! // Load migrations from directory
//! let migrations = load_migrations(Path::new("./migrations")).unwrap();
//!
//! // Apply migrations to build schema
//! let schema = apply_migrations(&migrations, Dialect::PostgreSQL).unwrap();
//!
//! // Analyze a query
//! let analyzer = QueryAnalyzer::new(&schema, Dialect::PostgreSQL);
//! let result = analyzer.analyze("SELECT id, name FROM users").unwrap();
//!
//! for col in result.columns {
//!     println!(
//!         "{}: {} (nullable: {})",
//!         col.name, col.data_type, col.nullable
//!     );
//! }
//! ```

// Re-export types
// Re-export analyzer
pub use sqlex_analyzer::{AnalyzeError, AnalyzeResult, QueryAnalyzer};
// Re-export migration
pub use sqlex_migration::{Migration, MigrationError, apply_migrations, load_migrations};
// Re-export schema
pub use sqlex_schema::{SchemaError, SchemaRegistry};
pub use sqlex_types::{ColumnDef, Dialect, ResultColumn, SqlType, TableDef};

// Re-export parser (for advanced use)
pub mod parser {
    pub use sqlex_parser::*;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_end_to_end() {
        // Create migrations programmatically
        let migrations = vec![
            Migration::new(
                1,
                "create_users",
                "CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR(100) NOT NULL)",
            ),
            Migration::new(
                2,
                "create_orders",
                "CREATE TABLE orders (id SERIAL PRIMARY KEY, user_id INT NOT NULL, amount DECIMAL(10,2))",
            ),
        ];

        // Build schema
        let schema = apply_migrations(&migrations, Dialect::PostgreSQL).unwrap();
        assert_eq!(schema.table_count(), 2);

        // Analyze query
        let analyzer = QueryAnalyzer::new(&schema, Dialect::PostgreSQL);

        // Simple select
        let result = analyzer.analyze("SELECT id, name FROM users").unwrap();
        assert_eq!(result.columns.len(), 2);
        assert_eq!(result.columns[0].name, "id");
        assert_eq!(result.columns[1].name, "name");

        // Join query
        let result = analyzer
            .analyze("SELECT u.name, o.amount FROM users u JOIN orders o ON u.id = o.user_id")
            .unwrap();
        assert_eq!(result.columns.len(), 2);
    }
}
