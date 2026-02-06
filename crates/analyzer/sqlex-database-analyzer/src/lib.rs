use sqlex_analyzer::{Analyzer, Result};
use sqlex_common::dialect::Dialect;

mod container_pool;
mod docker_raw;
pub mod mysql;
pub mod postgres;
pub mod sqlite;
mod utils;

pub async fn new_database_analyzer(dialect: Dialect) -> Result<Box<dyn Analyzer>> {
    match dialect {
        Dialect::Postgres => {
            let analyzer = postgres::PostgresDatabaseAnalyzer::new().await?;
            Ok(Box::new(analyzer) as Box<dyn Analyzer>)
        },
        Dialect::MySQL => {
            let analyzer = mysql::MySqlDatabaseAnalyzer::new().await?;
            Ok(Box::new(analyzer) as Box<dyn Analyzer>)
        },
        Dialect::SQLite => {
            let analyzer = sqlite::SqliteDatabaseAnalyzer::new().await?;
            Ok(Box::new(analyzer) as Box<dyn Analyzer>)
        },
    }
}
