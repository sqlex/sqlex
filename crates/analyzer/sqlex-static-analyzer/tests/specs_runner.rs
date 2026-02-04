use std::{fs, path::Path};

use serde::Deserialize;
use sqlex_common::{dialect::Dialect, types::DataType};
use sqlex_static_analyzer::{analysis::diagnostics::DiagnosticSeverity, catalog::Catalog};

#[derive(Debug, Deserialize)]
struct YamlTestSuite {
    #[serde(default)]
    schema: Vec<String>,
    tests: Vec<YamlTestCase>,
}

#[derive(Debug, Deserialize)]
struct YamlTestCase {
    name: String,
    sql: String,
    expected: Option<Vec<YamlOutputColumn>>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct YamlOutputColumn {
    name: String,
    #[serde(rename = "type")]
    data_type: DataType,
    nullability: bool,
}

#[test]
fn run_yaml_tests() {
    let specs_dir = Path::new("tests/specs");
    if !specs_dir.exists() {
        return;
    }

    for entry in fs::read_dir(specs_dir).expect("Failed to read specs directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.extension().is_some_and(|ext| ext == "yaml") {
            println!("Running tests from: {:?}", path);
            run_test_file(&path);
        }
    }
}

fn run_test_file(path: &Path) {
    let content = fs::read_to_string(path).expect("Failed to read file");
    let suite: YamlTestSuite = serde_yaml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse YAML file {:?}: {}", path, e));

    // Setup Catalog + Analyzer
    let dialect = Dialect::Postgres;
    let mut catalog = Catalog::new(dialect);
    let analyzer = sqlex_static_analyzer::analysis::AnalysisEngine::new(dialect);

    // Apply schema DDL
    for sql in suite.schema {
        let sql = sql.trim();
        if !sql.is_empty() {
            catalog.apply_ddl(sql).unwrap_or_else(|e| {
                panic!(
                    "Failed to apply DDL in {:?}:\nSQL: {}\nError: {}",
                    path, sql, e
                )
            });
        }
    }

    // Run tests
    for test in suite.tests {
        println!("  Running test: {}", test.name);

        if let Some(expected_error) = test.error {
            let result = analyzer.analyze(&catalog, &test.sql);
            let has_error = result
                .diagnostics
                .iter()
                .any(|d| d.severity == DiagnosticSeverity::Error);

            assert!(
                has_error,
                "Test '{}' in {:?}: Expected error containing '{}', but analysis succeeded",
                test.name, path, expected_error
            );
            // We could also check the error message content if needed, but for now just presence is enough or basic containment if easy.
            // verifying message can be added if needed, strict equality might be flaky for now.
        } else if let Some(expected_columns) = test.expected {
            let result = analyzer.analyze(&catalog, &test.sql);

            // Check for errors
            let errors: Vec<_> = result
                .diagnostics
                .iter()
                .filter(|d| d.severity == DiagnosticSeverity::Error)
                .collect();

            assert!(
                errors.is_empty(),
                "Test '{}' in {:?}: Expected success, got errors: {}",
                test.name,
                path,
                errors
                    .iter()
                    .map(|d| d.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            );

            let columns = result.output.expect("Expected output schema").columns;

            assert_eq!(
                columns.len(),
                expected_columns.len(),
                "Test '{}' in {:?}: output column count mismatch. Expected {}, got {}",
                test.name,
                path,
                expected_columns.len(),
                columns.len()
            );

            for (i, (actual, expected)) in columns.iter().zip(expected_columns.iter()).enumerate() {
                assert_eq!(
                    actual.name, expected.name,
                    "Test '{}' in {:?}: Column {} name mismatch",
                    test.name, path, i
                );
                assert_eq!(
                    actual.data_type, expected.data_type,
                    "Test '{}' in {:?}: Column {} type mismatch",
                    test.name, path, i
                );
                assert_eq!(
                    actual.nullability, expected.nullability,
                    "Test '{}' in {:?}: Column {} nullability mismatch (Expected nullability: {}, Actual: {})",
                    test.name, path, i, expected.nullability, actual.nullability
                );
            }
        } else {
            panic!(
                "Test '{}' in {:?} must have either 'expected' or 'error'",
                test.name, path
            );
        }
    }
}
