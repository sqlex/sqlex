use sqlex_analyzer::{Analyzer, Result};
use sqlex_common::Dialect;

pub mod mysql;
pub mod postgres;
pub mod sqlite;
mod utils;

pub use mysql::MySqlDatabaseAnalyzer;
pub use postgres::PostgresDatabaseAnalyzer;
pub use sqlite::SqliteDatabaseAnalyzer;

pub async fn new_database_analyzer(dialect: Dialect) -> Result<Box<dyn Analyzer>> {
    match dialect {
        Dialect::Postgres => {
            let analyzer = PostgresDatabaseAnalyzer::new().await?;
            Ok(Box::new(analyzer))
        },
        Dialect::MySQL => {
            let analyzer = MySqlDatabaseAnalyzer::new().await?;
            Ok(Box::new(analyzer))
        },
        Dialect::SQLite => {
            let analyzer = SqliteDatabaseAnalyzer::new().await?;
            Ok(Box::new(analyzer))
        },
    }
}
