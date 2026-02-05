use std::{
    env,
    path::{Path, PathBuf},
    process,
};

use serde::Deserialize;
use sqlex_analyzer::Analyzer;
use sqlex_common::{dialect::Dialect, types::DataType};
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

const SPECS_DIR: &str = "tests/specs";

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{}", err);
        process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let filter = parse_args()?;
    let specs_dir = Path::new(SPECS_DIR);
    let specs_meta = tokio_fs::metadata(specs_dir).await;
    let is_dir = match specs_meta {
        Ok(meta) => meta.is_dir(),
        Err(_) => false,
    };
    if !is_dir {
        return Ok(());
    }

    let mut files = Vec::new();
    collect_yaml_files(specs_dir, &mut files).await;
    files.sort_by(|a, b| {
        let a_rel = a.strip_prefix(specs_dir).unwrap_or(a).to_string_lossy();
        let b_rel = b.strip_prefix(specs_dir).unwrap_or(b).to_string_lossy();
        a_rel.cmp(&b_rel)
    });

    let total_files = files.len();
    let mut matched_files = Vec::new();
    for path in files {
        let rel_path = path.strip_prefix(specs_dir).unwrap_or(&path);
        if matches_filter(filter.as_deref(), rel_path) {
            matched_files.push(path);
        }
    }

    if let Some(filter_value) = filter.as_deref() {
        println!("Specs filter: {}", filter_value);
        println!("Matched {}/{} spec files", matched_files.len(), total_files);
        if matched_files.is_empty() {
            return Err(format!("No spec files matched filter '{}'.", filter_value));
        }
    }

    for path in matched_files {
        let display_path = path
            .strip_prefix(specs_dir)
            .unwrap_or(&path)
            .display()
            .to_string();
        println!("Running tests from: {}", display_path);
        run_test_file(&path, specs_dir).await;
    }

    Ok(())
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

fn parse_args() -> Result<Option<String>, String> {
    let mut args = env::args().skip(1);
    let mut filter: Option<String> = None;

    while let Some(arg) = args.next() {
        if arg == "-h" || arg == "--help" {
            print_usage();
            process::exit(0);
        }

        if arg == "--specs" {
            let value = args
                .next()
                .ok_or_else(|| "Missing value for --specs".to_string())?;
            if filter.is_some() {
                return Err("Duplicate --specs argument".to_string());
            }
            filter = Some(value);
            continue;
        }

        if let Some(value) = arg.strip_prefix("--specs=") {
            if value.is_empty() {
                return Err("Missing value for --specs".to_string());
            }
            if filter.is_some() {
                return Err("Duplicate --specs argument".to_string());
            }
            filter = Some(value.to_string());
            continue;
        }

        return Err(format!("Unknown argument: {}", arg));
    }

    let normalized = filter.map(|value| normalize_filter(&value));
    let normalized = match normalized {
        Some(value) if value.is_empty() => None,
        other => other,
    };
    Ok(normalized)
}

fn print_usage() {
    println!("Usage:");
    println!("  cargo test -p sqlex-static-analyzer --test specs_runner");
    println!("  cargo test -p sqlex-static-analyzer --test specs_runner -- --specs <path-prefix>");
    println!();
    println!("Examples:");
    println!("  --specs mysql/agg");
    println!("  --specs mysql/agg/basic");
    println!("  --specs tests/specs/mysql");
}

fn normalize_filter(raw: &str) -> String {
    let mut value = raw.trim().replace('\\', "/");
    value = value.trim_matches('/').to_string();
    if let Some(stripped) = value.strip_prefix("./") {
        value = stripped.to_string();
    }
    if value == "tests/specs" {
        value.clear();
        return value;
    }
    if let Some(stripped) = value.strip_prefix("tests/specs/") {
        value = stripped.to_string();
    }
    value
}

fn split_components(value: &str) -> Vec<&str> {
    value.split('/').filter(|part| !part.is_empty()).collect()
}

fn is_prefix(prefix: &[&str], path: &[&str]) -> bool {
    if prefix.len() > path.len() {
        return false;
    }
    prefix.iter().zip(path.iter()).all(|(a, b)| a == b)
}

fn matches_filter(filter: Option<&str>, rel_path: &Path) -> bool {
    let filter = match filter {
        Some(value) if !value.is_empty() => value,
        _ => return true,
    };

    let filter_components = split_components(filter);
    let rel_path_string = rel_path.to_string_lossy().replace('\\', "/");
    let rel_components = split_components(&rel_path_string);

    if is_prefix(&filter_components, &rel_components) {
        return true;
    }

    if rel_path.extension().is_some_and(|ext| ext == "yaml") {
        let no_ext_path = rel_path.with_extension("");
        let no_ext_string = no_ext_path.to_string_lossy().replace('\\', "/");
        let no_ext_components = split_components(&no_ext_string);
        if is_prefix(&filter_components, &no_ext_components) {
            return true;
        }
    }

    false
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
            let static_result = static_analyzer.execute(sql).await;
            let db_result = db_analyzer.execute(sql).await;

            match (db_result, static_result) {
                (Ok(_), Ok(_)) => {},
                (Err(db_err), Err(static_err)) => {
                    println!("  Migration result: db=ERROR, static=ERROR (match)");
                    println!("    SQL: {}", sql);
                    println!("    DB error: {}", db_err);
                    println!("    Static error: {}", static_err);
                },
                (Err(db_err), Ok(_)) => {
                    println!("  Migration result: db=ERROR, static=OK (mismatch)");
                    println!("    SQL: {}", sql);
                    println!("    DB error: {}", db_err);
                    println!("    Static error: <none>");
                    panic!(
                        "Migration in {} failed on database analyzer, but static analyzer succeeded.",
                        display_path
                    );
                },
                (Ok(_), Err(static_err)) => {
                    println!("  Migration result: db=OK, static=ERROR (mismatch)");
                    println!("    SQL: {}", sql);
                    println!("    DB error: <none>");
                    println!("    Static error: {}", static_err);
                    panic!(
                        "Migration in {} succeeded on database analyzer, but static analyzer failed.",
                        display_path
                    );
                },
            }
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
            (Err(db_err), Err(static_err)) => {
                println!("  Result: db=ERROR, static=ERROR (match)");
                println!("    DB error: {}", db_err);
                println!("    Static error: {}", static_err);
            },
            (Err(db_err), Ok(_)) => {
                println!("  Result: db=ERROR, static=OK (mismatch)");
                println!("    DB error: {}", db_err);
                println!("    Static error: <none>");
                panic!(
                    "Test '{}' in {}: Database analyzer failed, but static analyzer succeeded",
                    test.name, display_path
                );
            },
            (Ok(_), Err(static_err)) => {
                println!("  Result: db=OK, static=ERROR (mismatch)");
                println!("    DB error: <none>");
                println!("    Static error: {}", static_err);
                panic!(
                    "Test '{}' in {}: Database analyzer succeeded, but static analyzer failed",
                    test.name, display_path
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
                    // SQLite returns "null" type for aggregate functions on empty tables,
                    // which cannot be accurately mapped to a concrete type. Skip type
                    // comparison when db_col.data_type is Custom("null").
                    let skip_type_check = suite.dialect == Dialect::SQLite
                        && matches!(&db_col.data_type, DataType::Custom(s) if s == "null");
                    if !skip_type_check {
                        assert_eq!(
                            static_col.data_type, db_col.data_type,
                            "Test '{}' in {}: Column {} type mismatch (Expected: {:?}, Actual: {:?})",
                            test.name, display_path, i, db_col.data_type, static_col.data_type
                        );
                    }
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
