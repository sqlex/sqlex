use sqlex_analyzer::{Analyzer, Result};
use sqlex_common::DatabaseType;

pub mod mysql;
pub mod postgres;
pub mod sqlite;
mod utils;

pub use mysql::MySqlDatabaseAnalyzer;
pub use postgres::PostgresDatabaseAnalyzer;
pub use sqlite::SqliteDatabaseAnalyzer;

pub async fn new_database_analyzer(db_type: DatabaseType) -> Result<Box<dyn Analyzer>> {
    match db_type {
        DatabaseType::Postgres => {
            let analyzer = PostgresDatabaseAnalyzer::new().await?;
            Ok(Box::new(analyzer))
        },
        DatabaseType::MySQL => {
            let analyzer = MySqlDatabaseAnalyzer::new().await?;
            Ok(Box::new(analyzer))
        },
        DatabaseType::SQLite => {
            let analyzer = SqliteDatabaseAnalyzer::new().await?;
            Ok(Box::new(analyzer))
        },
    }
}
