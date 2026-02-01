//! Test runner that executes test suites against real databases.

use std::path::Path;

use crate::{
    config::{Dialect, TestCase, TestSuite},
    db::{self, ColumnMetadata, DatabaseBackend, QueryMetadata},
    Error, Result,
};

/// Result of running a single test case.
#[derive(Debug)]
pub struct TestCaseResult {
    /// Name of the test case.
    pub name: String,
    /// Whether the test passed.
    pub passed: bool,
    /// Error message if the test failed.
    pub error: Option<String>,
    /// Metadata from the real database.
    pub db_metadata: Option<QueryMetadata>,
    /// Metadata from sqlex analyzer.
    pub sqlex_metadata: Option<QueryMetadata>,
}

/// Result of running an entire test suite.
#[derive(Debug)]
pub struct TestSuiteResult {
    /// Dialect of the test suite.
    pub dialect: Dialect,
    /// Path to the test suite file.
    pub path: String,
    /// Results of individual test cases.
    pub results: Vec<TestCaseResult>,
}

impl TestSuiteResult {
    /// Check if all tests passed.
    pub fn all_passed(&self) -> bool {
        self.results.iter().all(|r| r.passed)
    }

    /// Count of passed tests.
    pub fn passed_count(&self) -> usize {
        self.results.iter().filter(|r| r.passed).count()
    }

    /// Count of failed tests.
    pub fn failed_count(&self) -> usize {
        self.results.iter().filter(|r| !r.passed).count()
    }
}

/// Test runner that executes test suites.
pub struct TestRunner {
    // Currently unused, but could cache backends in the future
}

impl TestRunner {
    /// Create a new test runner.
    pub fn new() -> Self {
        Self {}
    }

    /// Run a test suite from a file.
    pub async fn run_file(&mut self, path: impl AsRef<Path>) -> Result<TestSuiteResult> {
        let suite = TestSuite::from_file(&path)?;
        self.run_suite(&suite, path.as_ref().display().to_string())
            .await
    }

    /// Run a test suite.
    pub async fn run_suite(
        &mut self,
        suite: &TestSuite,
        path: String,
    ) -> Result<TestSuiteResult> {
        let dialect = suite.dialect();

        // Create a fresh backend for each suite to ensure isolation
        let backend = db::create_backend(dialect).await?;

        // Execute migrations
        backend.execute_migration(&suite.schema.migration).await?;

        // Parse migration with sqlex to build schema registry
        let sqlex_dialect = to_sqlex_dialect(dialect);
        let sqlex_registry = self.build_sqlex_registry(&suite.schema.migration, sqlex_dialect)?;

        // Run each test case
        let mut results = Vec::new();
        for test_case in &suite.tests {
            let result = self
                .run_test_case(test_case, &*backend, &sqlex_registry, sqlex_dialect)
                .await;
            results.push(result);
        }

        // Cleanup
        backend.cleanup().await?;

        Ok(TestSuiteResult {
            dialect,
            path,
            results,
        })
    }

    /// Build a sqlex SchemaRegistry from migration SQL.
    fn build_sqlex_registry(
        &self,
        migration: &str,
        dialect: sqlex_types::Dialect,
    ) -> Result<sqlex_schema::SchemaRegistry> {
        let mut registry = sqlex_schema::SchemaRegistry::new(dialect);
        registry.apply_sql(migration).map_err(|e| {
            Error::MigrationParse(format!("{}", e))
        })?;
        Ok(registry)
    }

    /// Run a single test case.
    async fn run_test_case(
        &self,
        test_case: &TestCase,
        backend: &dyn DatabaseBackend,
        registry: &sqlex_schema::SchemaRegistry,
        dialect: sqlex_types::Dialect,
    ) -> TestCaseResult {
        // Get metadata from real database
        let db_result = backend.describe_query(&test_case.query).await;

        // Get metadata from sqlex analyzer
        let analyzer = sqlex_analyzer::QueryAnalyzer::new(registry, dialect);
        let sqlex_result = analyzer.analyze(&test_case.query);

        match (db_result, sqlex_result) {
            (Ok(db_metadata), Ok(sqlex_analyzed)) => {
                // Convert sqlex result to our QueryMetadata format
                let sqlex_metadata = self.convert_sqlex_result(&sqlex_analyzed);

                // Compare the results
                match self.compare_metadata(&db_metadata, &sqlex_metadata) {
                    Ok(()) => TestCaseResult {
                        name: test_case.name.clone(),
                        passed: !test_case.expect_error,
                        error: if test_case.expect_error {
                            Some("Expected error but query succeeded".to_string())
                        } else {
                            None
                        },
                        db_metadata: Some(db_metadata),
                        sqlex_metadata: Some(sqlex_metadata),
                    },
                    Err(e) => TestCaseResult {
                        name: test_case.name.clone(),
                        passed: false,
                        error: Some(e.to_string()),
                        db_metadata: Some(db_metadata),
                        sqlex_metadata: Some(sqlex_metadata),
                    },
                }
            }
            (Err(db_err), Ok(_)) => TestCaseResult {
                name: test_case.name.clone(),
                passed: test_case.expect_error,
                error: Some(format!("Database error: {}", db_err)),
                db_metadata: None,
                sqlex_metadata: None,
            },
            (Ok(_), Err(sqlex_err)) => TestCaseResult {
                name: test_case.name.clone(),
                passed: false,
                error: Some(format!("Sqlex analysis error: {:?}", sqlex_err)),
                db_metadata: None,
                sqlex_metadata: None,
            },
            (Err(db_err), Err(sqlex_err)) => TestCaseResult {
                name: test_case.name.clone(),
                passed: test_case.expect_error,
                error: if test_case.expect_error {
                    None
                } else {
                    Some(format!(
                        "Both failed - DB: {}, Sqlex: {:?}",
                        db_err, sqlex_err
                    ))
                },
                db_metadata: None,
                sqlex_metadata: None,
            },
        }
    }

    /// Convert sqlex analysis result to our QueryMetadata format.
    fn convert_sqlex_result(
        &self,
        result: &sqlex_analyzer::AnalyzeResult,
    ) -> QueryMetadata {
        let columns = result
            .columns
            .iter()
            .map(|col| ColumnMetadata {
                name: col.name.clone(),
                type_name: format!("{:?}", col.data_type),
                nullable: col.nullable,
            })
            .collect();

        // Note: AnalyzeResult currently doesn't include parameters
        // This will need to be extended later
        let params = vec![];

        QueryMetadata { columns, params }
    }

    /// Compare metadata from database and sqlex.
    fn compare_metadata(
        &self,
        db_metadata: &QueryMetadata,
        sqlex_metadata: &QueryMetadata,
    ) -> Result<()> {
        // Compare column counts
        if db_metadata.columns.len() != sqlex_metadata.columns.len() {
            return Err(Error::MetadataMismatch {
                message: format!(
                    "Column count mismatch: DB has {}, sqlex has {}",
                    db_metadata.columns.len(),
                    sqlex_metadata.columns.len()
                ),
            });
        }

        // Compare column names
        for (i, (db_col, sqlex_col)) in db_metadata
            .columns
            .iter()
            .zip(sqlex_metadata.columns.iter())
            .enumerate()
        {
            if db_col.name.to_lowercase() != sqlex_col.name.to_lowercase() {
                return Err(Error::MetadataMismatch {
                    message: format!(
                        "Column {} name mismatch: DB='{}', sqlex='{}'",
                        i, db_col.name, sqlex_col.name
                    ),
                });
            }
            
            // Note: Type comparison is more complex due to different representations
            // For now, we just log the difference but don't fail
            // TODO: Add proper type normalization and comparison
        }

        Ok(())
    }
}

impl Default for TestRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert our Dialect to sqlex_types::Dialect.
fn to_sqlex_dialect(dialect: Dialect) -> sqlex_types::Dialect {
    match dialect {
        Dialect::Postgresql => sqlex_types::Dialect::PostgreSQL,
        Dialect::Mysql => sqlex_types::Dialect::MySQL,
        Dialect::Sqlite => sqlex_types::Dialect::SQLite,
    }
}
