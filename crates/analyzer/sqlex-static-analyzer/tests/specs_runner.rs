use std::path::{Path, PathBuf};

use serde::Deserialize;
use sqlex_analyzer::Analyzer;
use sqlex_common::{dialect::Dialect, types::DataType};
use sqlex_static_analyzer::StaticAnalyzer;
use tokio::fs as tokio_fs;

#[derive(Debug, Deserialize)]
struct YamlTestSuite {
    dialect: Dialect,
    #[serde(default)]
    migrations: Vec<String>,
    #[serde(default)]
    tables: Vec<YamlTable>,
    queries: Vec<YamlQuery>,
}

#[derive(Debug, Deserialize)]
struct YamlTable {
    name: String,
    columns: Vec<YamlOutputColumn>,
}

#[derive(Debug, Deserialize)]
struct YamlQuery {
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

#[tokio::test]
async fn run_specs_tests() {
    let specs_dir = Path::new("tests/specs");
    let specs_meta = tokio_fs::metadata(specs_dir).await;
    let is_dir = match specs_meta {
        Ok(meta) => meta.is_dir(),
        Err(_) => false,
    };
    if !is_dir {
        return;
    }

    let mut files = Vec::new();
    collect_yaml_files(specs_dir, &mut files).await;
    for path in files {
        let display_path = path
            .strip_prefix(specs_dir)
            .unwrap_or(&path)
            .display()
            .to_string();
        println!("Running tests from: {}", display_path);
        run_test_file(&path, specs_dir).await;
    }
}

async fn collect_yaml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let mut entries = match tokio_fs::read_dir(&current).await {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "yaml") {
                out.push(path);
            }
        }
    }
}

async fn run_test_file(path: &Path, specs_dir: &Path) {
    let display_path = path
        .strip_prefix(specs_dir)
        .unwrap_or(path)
        .display()
        .to_string();
    let content = tokio_fs::read_to_string(path)
        .await
        .expect("Failed to read file");
    let suite: YamlTestSuite = serde_yaml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse YAML file {}: {}", display_path, e));

    let mut analyzer = StaticAnalyzer::new(suite.dialect);

    // Apply migrations
    for sql in suite.migrations {
        let sql = sql.trim();
        if !sql.is_empty() {
            analyzer.execute(sql).await.unwrap_or_else(|e| {
                panic!(
                    "Failed to apply migration in {}:\nSQL: {}\nError: {}",
                    display_path, sql, e
                )
            });
        }
    }

    if !suite.tables.is_empty() {
        let mut actual_tables = analyzer.get_all_tables().await.unwrap_or_else(|e| {
            panic!(
                "Failed to get tables for schema validation in {}: {}",
                display_path, e
            )
        });
        actual_tables.sort_by(|a, b| a.name.cmp(&b.name));

        let mut expected_tables = suite.tables;
        expected_tables.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(
            actual_tables.len(),
            expected_tables.len(),
            "Schema in {}: table count mismatch. Expected {} tables, got {}. Expected: [{}], Actual: [{}]",
            display_path,
            expected_tables.len(),
            actual_tables.len(),
            expected_tables
                .iter()
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            actual_tables
                .iter()
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );

        for (actual, expected) in actual_tables.iter().zip(expected_tables.iter()) {
            assert_eq!(
                actual.name, expected.name,
                "Schema in {}: table name mismatch. Expected {}, got {}",
                display_path, expected.name, actual.name
            );
            assert_eq!(
                actual.columns.len(),
                expected.columns.len(),
                "Schema in {}: column count mismatch for table {}. Expected {} columns, got {}. Expected: [{}], Actual: [{}]",
                display_path,
                expected.name,
                expected.columns.len(),
                actual.columns.len(),
                expected
                    .columns
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                actual
                    .columns
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            for (i, (actual_col, expected_col)) in actual
                .columns
                .iter()
                .zip(expected.columns.iter())
                .enumerate()
            {
                assert_eq!(
                    actual_col.name, expected_col.name,
                    "Schema in {}: column {} name mismatch for table {} (Expected: {}, Actual: {})",
                    display_path, i, expected.name, expected_col.name, actual_col.name
                );
                assert_eq!(
                    actual_col.data_type, expected_col.data_type,
                    "Schema in {}: column {} type mismatch for table {} (Expected: {:?}, Actual: {:?})",
                    display_path, i, expected.name, expected_col.data_type, actual_col.data_type
                );
                assert_eq!(
                    actual_col.nullability,
                    expected_col.nullability,
                    "Schema in {}: column {} nullability mismatch for table {} (Expected: {}, Actual: {})",
                    display_path,
                    i,
                    expected.name,
                    expected_col.nullability,
                    actual_col.nullability
                );
            }
        }
    }

    // Run queries
    for test in suite.queries {
        println!("  Running test: {}", test.name);

        if let Some(expected_error) = test.error {
            let result = analyzer.analyze(&test.sql).await;
            assert!(
                result.is_err(),
                "Test '{}' in {}: Expected error containing '{}', but analysis succeeded",
                test.name,
                display_path,
                expected_error
            );
            let message = result.unwrap_err().to_string();
            assert!(
                message.contains(&expected_error),
                "Test '{}' in {}: Expected error containing '{}', got '{}'",
                test.name,
                display_path,
                expected_error,
                message
            );
        } else if let Some(expected_columns) = test.expected {
            let result = analyzer.analyze(&test.sql).await.unwrap_or_else(|e| {
                panic!(
                    "Test '{}' in {}: Expected success, got error: {}",
                    test.name, display_path, e
                )
            });
            let columns = result.columns;

            assert_eq!(
                columns.len(),
                expected_columns.len(),
                "Test '{}' in {}: output column count mismatch. Expected {}, got {}",
                test.name,
                display_path,
                expected_columns.len(),
                columns.len()
            );

            for (i, (actual, expected)) in columns.iter().zip(expected_columns.iter()).enumerate() {
                assert_eq!(
                    actual.name, expected.name,
                    "Test '{}' in {}: Column {} name mismatch",
                    test.name, display_path, i
                );
                assert_eq!(
                    actual.data_type, expected.data_type,
                    "Test '{}' in {}: Column {} type mismatch",
                    test.name, display_path, i
                );
                assert_eq!(
                    actual.nullability, expected.nullability,
                    "Test '{}' in {}: Column {} nullability mismatch (Expected nullability: {}, Actual: {})",
                    test.name, display_path, i, expected.nullability, actual.nullability
                );
            }
        } else {
            panic!(
                "Test '{}' in {} must have either 'expected' or 'error'",
                test.name, display_path
            );
        }
    }
}
