use async_trait::async_trait;
use sqlex_analyzer::{Analyzer, Result};
use sqlex_common::{
    dialect::Dialect,
    types::{ColumnInfo, ResultSet, Table},
};
use sqlex_database_analyzer::new_database_analyzer;
use sqlex_static_analyzer::StaticAnalyzer;
use tracing::warn;

mod warning;

pub use crate::warning::AnalysisWarning;

/// Hybrid analyzer that combines static analyzer and database analyzer.
///
/// Strategy:
/// - Database analyzer: Most accurate for column names and types
/// - Static analyzer: Accurate for nullability and cardinality inference
/// - SQLite special case: Only column names are accurate from database analyzer
///
/// When both analyzers provide results for the same domain and they differ,
/// the database analyzer takes precedence, but a warning is emitted.
pub struct HybridAnalyzer {
    dialect: Dialect,
    static_analyzer: StaticAnalyzer,
    db_analyzer: Box<dyn Analyzer>,
}

impl HybridAnalyzer {
    /// Creates a new hybrid analyzer for the given dialect.
    pub async fn new(dialect: Dialect) -> Result<Self> {
        let static_analyzer = StaticAnalyzer::new(dialect);
        let db_analyzer = new_database_analyzer(dialect).await?;

        Ok(Self {
            dialect,
            static_analyzer,
            db_analyzer,
        })
    }

    /// Merges results from static and database analyzers.
    fn merge_results(
        &self,
        static_result: ResultSet,
        db_result: ResultSet,
    ) -> (ResultSet, Vec<AnalysisWarning>) {
        let mut warnings = Vec::new();

        // Check column count mismatch
        if static_result.columns.len() != db_result.columns.len() {
            warnings.push(AnalysisWarning::ColumnCountMismatch {
                static_count: static_result.columns.len(),
                db_count: db_result.columns.len(),
            });
            // If counts differ, trust database analyzer completely
            return (db_result, warnings);
        }

        let merged_columns = static_result
            .columns
            .into_iter()
            .zip(db_result.columns)
            .enumerate()
            .map(|(idx, (static_col, db_col))| {
                self.merge_column(idx, static_col, db_col, &mut warnings)
            })
            .collect();

        let merged = ResultSet {
            columns: merged_columns,
            // Use static analyzer's cardinality (database analyzer doesn't provide it)
            cardinality: static_result.cardinality,
        };

        (merged, warnings)
    }

    /// Merges a single column from both analyzers.
    fn merge_column(
        &self,
        index: usize,
        static_col: ColumnInfo,
        db_col: ColumnInfo,
        warnings: &mut Vec<AnalysisWarning>,
    ) -> ColumnInfo {
        // Column name: prefer database analyzer
        let name = if static_col.name != db_col.name {
            warnings.push(AnalysisWarning::ColumnNameMismatch {
                index,
                static_name: static_col.name.clone(),
                db_name: db_col.name.clone(),
            });
            db_col.name
        } else {
            db_col.name
        };

        // Data type and nullability depend on dialect
        let (data_type, nullability) = match self.dialect {
            Dialect::SQLite => {
                // SQLite: Only column names are accurate from database analyzer
                // Use static analyzer for type and nullability
                (static_col.data_type, static_col.nullability)
            },
            Dialect::MySQL | Dialect::Postgres => {
                // MySQL/Postgres: Database analyzer is accurate for types
                // Static analyzer is accurate for nullability

                // Check for type mismatch (exact match required)
                if static_col.data_type != db_col.data_type {
                    warnings.push(AnalysisWarning::DataTypeMismatch {
                        index,
                        column_name: name.clone(),
                        static_type: static_col.data_type.clone(),
                        db_type: db_col.data_type.clone(),
                    });
                }

                // Use database type, static nullability
                (db_col.data_type, static_col.nullability)
            },
        };

        ColumnInfo {
            name,
            data_type,
            nullability,
        }
    }

    /// Validates tables from both analyzers and returns warnings.
    fn validate_tables(
        &self,
        static_tables: &[Table],
        db_tables: &[Table],
    ) -> Vec<AnalysisWarning> {
        let mut warnings = Vec::new();

        // Check table count mismatch
        if static_tables.len() != db_tables.len() {
            warnings.push(AnalysisWarning::TableCountMismatch {
                static_count: static_tables.len(),
                db_count: db_tables.len(),
            });
        }

        // For each database table, find corresponding static table
        for db_table in db_tables {
            match static_tables.iter().find(|t| t.name == db_table.name) {
                Some(static_table) => {
                    // Validate columns for this table
                    self.validate_table_columns(static_table, db_table, &mut warnings);
                },
                None => {
                    warnings.push(AnalysisWarning::TableMissing {
                        table_name: db_table.name.clone(),
                        found_in_static: false,
                    });
                },
            }
        }

        // Check for tables in static analyzer but not in database
        for static_table in static_tables {
            if !db_tables.iter().any(|t| t.name == static_table.name) {
                warnings.push(AnalysisWarning::TableMissing {
                    table_name: static_table.name.clone(),
                    found_in_static: true,
                });
            }
        }

        warnings
    }

    /// Validates columns for a specific table.
    fn validate_table_columns(
        &self,
        static_table: &Table,
        db_table: &Table,
        warnings: &mut Vec<AnalysisWarning>,
    ) {
        // Check column count mismatch
        if static_table.columns.len() != db_table.columns.len() {
            warnings.push(AnalysisWarning::ColumnCountMismatch {
                static_count: static_table.columns.len(),
                db_count: db_table.columns.len(),
            });
            return; // Cannot compare columns if counts differ
        }

        // Compare each column
        for (i, (static_col, db_col)) in static_table
            .columns
            .iter()
            .zip(db_table.columns.iter())
            .enumerate()
        {
            // Check column name
            if static_col.name != db_col.name {
                warnings.push(AnalysisWarning::ColumnNameMismatch {
                    index: i,
                    static_name: static_col.name.clone(),
                    db_name: db_col.name.clone(),
                });
            }

            // Check data type
            if static_col.data_type != db_col.data_type {
                warnings.push(AnalysisWarning::DataTypeMismatch {
                    index: i,
                    column_name: db_col.name.clone(),
                    static_type: static_col.data_type.clone(),
                    db_type: db_col.data_type.clone(),
                });
            }

            // Check nullability
            if static_col.nullability != db_col.nullability {
                warnings.push(AnalysisWarning::NullabilityMismatch {
                    index: i,
                    column_name: db_col.name.clone(),
                    static_nullability: static_col.nullability,
                    db_nullability: db_col.nullability,
                });
            }
        }
    }
}

#[async_trait]
impl Analyzer for HybridAnalyzer {
    async fn execute(&mut self, sql: &str) -> Result<()> {
        // Execute on both analyzers to keep them in sync
        self.static_analyzer.execute(sql).await?;
        self.db_analyzer.execute(sql).await?;
        Ok(())
    }

    async fn analyze(&self, sql: &str) -> Result<ResultSet> {
        // Get results from both analyzers
        let static_result = self.static_analyzer.analyze(sql).await?;
        let db_result = self.db_analyzer.analyze(sql).await?;

        // Merge results and emit warnings
        let (merged, warnings) = self.merge_results(static_result, db_result);

        // Log warnings
        for warning in warnings {
            warn!("{}", warning);
        }

        Ok(merged)
    }

    async fn get_all_tables(&self) -> Result<Vec<Table>> {
        // Get tables from both analyzers
        let static_tables = self.static_analyzer.get_all_tables().await?;
        let db_tables = self.db_analyzer.get_all_tables().await?;

        // Validate and detect inconsistencies
        let warnings = self.validate_tables(&static_tables, &db_tables);

        // Log warnings
        for warning in warnings {
            warn!("{}", warning);
        }

        // Return database analyzer's result
        Ok(db_tables)
    }
}

#[cfg(test)]
mod tests {
    use sqlex_common::types::{ColumnInfo, DataType};

    use super::*;

    /// Mock analyzer for testing
    struct MockAnalyzer;

    #[async_trait]
    impl Analyzer for MockAnalyzer {
        async fn execute(&mut self, _sql: &str) -> Result<()> {
            Ok(())
        }

        async fn analyze(&self, _sql: &str) -> Result<ResultSet> {
            Ok(ResultSet {
                columns: vec![],
                cardinality: sqlex_common::types::Cardinality::Unknown,
            })
        }

        async fn get_all_tables(&self) -> Result<Vec<Table>> {
            Ok(vec![])
        }
    }

    /// Helper function to create a test column
    fn make_column(name: &str, data_type: DataType, nullable: bool) -> ColumnInfo {
        ColumnInfo {
            name: name.to_string(),
            data_type,
            nullability: nullable,
        }
    }

    /// Helper function to create a test table
    fn make_table(name: &str, columns: Vec<ColumnInfo>) -> Table {
        Table {
            name: name.to_string(),
            columns,
        }
    }

    #[test]
    fn test_merge_column_mysql_matching() {
        let analyzer = HybridAnalyzer {
            dialect: Dialect::MySQL,
            static_analyzer: StaticAnalyzer::new(Dialect::MySQL),
            db_analyzer: Box::new(MockAnalyzer),
        };

        let static_col = make_column("id", DataType::Int, false);
        let db_col = make_column("id", DataType::Int, true);
        let mut warnings = Vec::new();

        let result = analyzer.merge_column(0, static_col, db_col, &mut warnings);

        // For MySQL: use db type, static nullability
        assert_eq!(result.name, "id");
        assert_eq!(result.data_type, DataType::Int);
        assert!(!result.nullability); // static nullability
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_merge_column_mysql_type_mismatch() {
        let analyzer = HybridAnalyzer {
            dialect: Dialect::MySQL,
            static_analyzer: StaticAnalyzer::new(Dialect::MySQL),
            db_analyzer: Box::new(MockAnalyzer),
        };

        let static_col = make_column("id", DataType::Int, false);
        let db_col = make_column("id", DataType::BigInt, true);
        let mut warnings = Vec::new();

        let result = analyzer.merge_column(0, static_col, db_col, &mut warnings);

        // Should use db type and emit warning
        assert_eq!(result.data_type, DataType::BigInt);
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            warnings[0],
            AnalysisWarning::DataTypeMismatch { .. }
        ));
    }

    #[test]
    fn test_merge_column_sqlite_uses_static_type() {
        let analyzer = HybridAnalyzer {
            dialect: Dialect::SQLite,
            static_analyzer: StaticAnalyzer::new(Dialect::SQLite),
            db_analyzer: Box::new(MockAnalyzer),
        };

        let static_col = make_column("name", DataType::Text, false);
        let db_col = make_column("name", DataType::Varchar, true);
        let mut warnings = Vec::new();

        let result = analyzer.merge_column(0, static_col, db_col, &mut warnings);

        // For SQLite: use static type and nullability
        assert_eq!(result.name, "name");
        assert_eq!(result.data_type, DataType::Text);
        assert!(!result.nullability);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_merge_column_name_mismatch() {
        let analyzer = HybridAnalyzer {
            dialect: Dialect::MySQL,
            static_analyzer: StaticAnalyzer::new(Dialect::MySQL),
            db_analyzer: Box::new(MockAnalyzer),
        };

        let static_col = make_column("user_id", DataType::Int, false);
        let db_col = make_column("userId", DataType::Int, true);
        let mut warnings = Vec::new();

        let result = analyzer.merge_column(0, static_col, db_col, &mut warnings);

        // Should use db name and emit warning
        assert_eq!(result.name, "userId");
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            warnings[0],
            AnalysisWarning::ColumnNameMismatch { .. }
        ));
    }

    #[test]
    fn test_validate_tables_count_mismatch() {
        let analyzer = HybridAnalyzer {
            dialect: Dialect::MySQL,
            static_analyzer: StaticAnalyzer::new(Dialect::MySQL),
            db_analyzer: Box::new(MockAnalyzer),
        };

        let static_tables = vec![make_table("users", vec![]), make_table("posts", vec![])];
        let db_tables = vec![make_table("users", vec![])];

        let warnings = analyzer.validate_tables(&static_tables, &db_tables);

        assert_eq!(warnings.len(), 2); // count mismatch + missing table
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, AnalysisWarning::TableCountMismatch { .. }))
        );
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, AnalysisWarning::TableMissing { .. }))
        );
    }

    #[test]
    fn test_validate_tables_missing_in_db() {
        let analyzer = HybridAnalyzer {
            dialect: Dialect::MySQL,
            static_analyzer: StaticAnalyzer::new(Dialect::MySQL),
            db_analyzer: Box::new(MockAnalyzer),
        };

        let static_tables = vec![make_table("users", vec![]), make_table("posts", vec![])];
        let db_tables = vec![make_table("users", vec![])];

        let warnings = analyzer.validate_tables(&static_tables, &db_tables);

        let missing_warnings: Vec<_> = warnings
            .iter()
            .filter_map(|w| match w {
                AnalysisWarning::TableMissing {
                    table_name,
                    found_in_static,
                } => Some((table_name.as_str(), *found_in_static)),
                _ => None,
            })
            .collect();

        assert_eq!(missing_warnings.len(), 1);
        assert_eq!(missing_warnings[0], ("posts", true));
    }
}
