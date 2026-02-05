use std::path::{Path, PathBuf};

use serde::Deserialize;
use sqlex_analyzer::Analyzer;
use sqlex_common::dialect::Dialect;
use sqlex_database_analyzer::new_database_analyzer;
use sqlex_static_analyzer::StaticAnalyzer;
use tokio::fs as tokio_fs;

#[derive(Debug, Deserialize)]
struct YamlTestSuite {
    dialect: Dialect,
    #[serde(default)]
    migrations: Vec<String>,
    queries: Vec<YamlQuery>,
}

#[derive(Debug, Deserialize)]
struct YamlQuery {
    name: String,
    sql: String,
    #[serde(default)]
    expected: Vec<YamlOutputColumn>,
}

#[derive(Debug, Deserialize)]
struct YamlOutputColumn {
    name: String,
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

    let mut static_analyzer = StaticAnalyzer::new(suite.dialect);
    let mut db_analyzer = new_database_analyzer(suite.dialect)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "Failed to create database analyzer for {}: {}",
                display_path, e
            )
        });

    // Apply migrations
    for sql in suite.migrations {
        let sql = sql.trim();
        if !sql.is_empty() {
            static_analyzer.execute(sql).await.unwrap_or_else(|e| {
                panic!(
                    "Failed to apply migration in {}:\nSQL: {}\nError: {}",
                    display_path, sql, e
                )
            });
            db_analyzer.execute(sql).await.unwrap_or_else(|e| {
                panic!(
                    "Failed to apply migration in {} (database analyzer):\nSQL: {}\nError: {}",
                    display_path, sql, e
                )
            });
        }
    }

    let mut static_tables = static_analyzer.get_all_tables().await.unwrap_or_else(|e| {
        panic!(
            "Failed to get tables for schema validation in {}: {}",
            display_path, e
        )
    });
    static_tables.sort_by(|a, b| a.name.cmp(&b.name));

    let mut db_tables = db_analyzer.get_all_tables().await.unwrap_or_else(|e| {
        panic!(
            "Failed to get tables for schema validation in {} (database analyzer): {}",
            display_path, e
        )
    });
    db_tables.sort_by(|a, b| a.name.cmp(&b.name));

    assert_eq!(
        static_tables.len(),
        db_tables.len(),
        "Schema in {}: table count mismatch. Expected {} tables, got {}. Expected: [{}], Actual: [{}]",
        display_path,
        db_tables.len(),
        static_tables.len(),
        db_tables
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        static_tables
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    for (actual, expected) in static_tables.iter().zip(db_tables.iter()) {
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
                actual_col.nullability, expected_col.nullability,
                "Schema in {}: column {} nullability mismatch for table {} (Expected: {}, Actual: {})",
                display_path, i, expected.name, expected_col.nullability, actual_col.nullability
            );
        }
    }

    // Run queries
    for test in suite.queries {
        println!("  Running test: {}", test.name);
        let db_result = db_analyzer.analyze(&test.sql).await;
        let static_result = static_analyzer.analyze(&test.sql).await;

        match (db_result, static_result) {
            (Err(db_err), Err(_)) => {
                println!(
                    "  Database analyzer failed for '{}': {} (static analyzer also failed, ok)",
                    test.name, db_err
                );
            },
            (Err(db_err), Ok(_)) => {
                panic!(
                    "Test '{}' in {}: Database analyzer failed ('{}'), but static analyzer succeeded",
                    test.name, display_path, db_err
                );
            },
            (Ok(_), Err(static_err)) => {
                panic!(
                    "Test '{}' in {}: Database analyzer succeeded, but static analyzer failed: {}",
                    test.name, display_path, static_err
                );
            },
            (Ok(db_result), Ok(static_result)) => {
                let db_columns = db_result.columns;
                let static_columns = static_result.columns;

                assert_eq!(
                    static_columns.len(),
                    db_columns.len(),
                    "Test '{}' in {}: output column count mismatch. Expected {}, got {}",
                    test.name,
                    display_path,
                    db_columns.len(),
                    static_columns.len()
                );

                assert_eq!(
                    test.expected.len(),
                    db_columns.len(),
                    "Test '{}' in {}: expected nullability count mismatch. Expected {}, got {}",
                    test.name,
                    display_path,
                    db_columns.len(),
                    test.expected.len()
                );

                for i in 0..db_columns.len() {
                    let db_col = &db_columns[i];
                    let static_col = &static_columns[i];
                    let expected_col = &test.expected[i];

                    assert_eq!(
                        static_col.name, db_col.name,
                        "Test '{}' in {}: Column {} name mismatch (Expected: {}, Actual: {})",
                        test.name, display_path, i, db_col.name, static_col.name
                    );
                    assert_eq!(
                        expected_col.name, db_col.name,
                        "Test '{}' in {}: Expected column {} name mismatch (Expected: {}, Actual: {})",
                        test.name, display_path, i, db_col.name, expected_col.name
                    );
                    assert_eq!(
                        static_col.data_type, db_col.data_type,
                        "Test '{}' in {}: Column {} type mismatch (Expected: {:?}, Actual: {:?})",
                        test.name, display_path, i, db_col.data_type, static_col.data_type
                    );
                    assert_eq!(
                        static_col.nullability,
                        expected_col.nullability,
                        "Test '{}' in {}: Column {} nullability mismatch (Expected: {}, Actual: {})",
                        test.name,
                        display_path,
                        i,
                        expected_col.nullability,
                        static_col.nullability
                    );
                }
            },
        }
    }
}
