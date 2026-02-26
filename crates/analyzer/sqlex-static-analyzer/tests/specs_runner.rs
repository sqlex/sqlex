use std::{
    collections::HashSet,
    env,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use sqlex_analyzer::Analyzer;
use sqlex_common::{
    dialect::Dialect,
    types::{Cardinality, DataType},
};
use sqlex_database_analyzer::new_database_analyzer;
use sqlex_static_analyzer::StaticAnalyzer;
use tokio::fs as tokio_fs;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct YamlTestSuite {
    dialects: Option<Vec<Dialect>>,
    #[serde(default)]
    migrations: Vec<YamlMigration>,
    queries: Vec<YamlQuery>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum YamlMigration {
    Sql(String),
    Case(YamlMigrationCase),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct YamlMigrationCase {
    name: Option<String>,
    sql: String,
    expected_error_code: Option<String>,
}

impl YamlMigration {
    fn name(&self, index: usize) -> String {
        match self {
            YamlMigration::Sql(_) => format!("migration_{}", index),
            YamlMigration::Case(case) => case
                .name
                .as_ref()
                .map_or_else(|| format!("migration_{}", index), ToString::to_string),
        }
    }

    fn sql(&self) -> &str {
        match self {
            YamlMigration::Sql(sql) => sql,
            YamlMigration::Case(case) => &case.sql,
        }
    }

    fn expected_error_code(&self) -> Option<&str> {
        match self {
            YamlMigration::Sql(_) => None,
            YamlMigration::Case(case) => case.expected_error_code.as_deref(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct YamlQuery {
    name: String,
    sql: String,
    cardinality: Option<Cardinality>,
    #[serde(default)]
    expected: Vec<YamlOutputColumn>,
    expected_error_code: Option<String>,
    tdd_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct YamlOutputColumn {
    name: String,
    nullability: bool,
}

const SPECS_DIR: &str = "tests/specs";

#[derive(Debug, Default)]
struct RunStats {
    total_files: usize,
    matched_files: usize,
    dialect_runs: usize,
    query_cases: usize,
    skipped_tdd_cases: usize,
}

#[tokio::test]
async fn specs_runner() {
    if let Err(error) = run().await {
        panic!("{}", error);
    }
}

async fn run() -> Result<(), String> {
    let filter = parse_filter_from_env();
    let run_tdd = should_run_tdd();
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

    let mut stats = RunStats {
        total_files: files.len(),
        ..RunStats::default()
    };

    let mut matched_files = Vec::new();
    for path in files {
        let rel_path = path.strip_prefix(specs_dir).unwrap_or(&path);
        if matches_filter(filter.as_deref(), rel_path) {
            matched_files.push(path);
        }
    }

    stats.matched_files = matched_files.len();

    if let Some(filter_value) = filter.as_deref() {
        println!("Specs filter: {}", filter_value);
        println!(
            "Matched {}/{} spec files",
            matched_files.len(),
            stats.total_files
        );
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
        run_test_file(&path, specs_dir, run_tdd, &mut stats).await?;
    }

    println!(
        "Specs summary: files={}/{}, dialect_runs={}, query_cases={}, skipped_tdd={}",
        stats.matched_files,
        stats.total_files,
        stats.dialect_runs,
        stats.query_cases,
        stats.skipped_tdd_cases
    );

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

fn parse_filter_from_env() -> Option<String> {
    let raw = env::var("SQLEX_SPECS_FILTER").ok()?;
    let normalized = normalize_filter(&raw);
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn should_run_tdd() -> bool {
    let raw = env::var("SQLEX_RUN_TDD").unwrap_or_else(|_| "0".to_string());
    matches!(raw.trim(), "1" | "true" | "TRUE" | "True")
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

/// Check if SQLite types are compatible.
/// SQLite's database analyzer may return TEXT for VARCHAR/CHAR types in query results.
/// Also returns "null" type for aggregate functions on empty tables.
fn sqlite_types_compatible(static_type: &DataType, db_type: &DataType) -> bool {
    // Exact match is always compatible
    if static_type == db_type {
        return true;
    }

    // SQLite returns "null" type for aggregate functions on empty tables
    if matches!(db_type, DataType::Custom(s) if s == "null") {
        return true;
    }

    // SQLite maps VARCHAR and CHAR to TEXT in query results
    matches!(
        (static_type, db_type),
        (DataType::Varchar, DataType::Text) | (DataType::Char, DataType::Text)
    )
}

fn all_dialects() -> [Dialect; 3] {
    [Dialect::MySQL, Dialect::Postgres, Dialect::SQLite]
}

fn extract_error_code(error: &impl std::fmt::Display) -> Option<String> {
    let rendered = error.to_string();
    let bracket_start = rendered.find('[')?;
    let payload = &rendered[(bracket_start + 1)..];
    let bracket_end_relative = payload.find(']')?;
    let payload = &payload[..bracket_end_relative];
    payload
        .rsplit(':')
        .next()
        .filter(|code| !code.is_empty())
        .map(ToString::to_string)
}

fn validate_suite(suite: &YamlTestSuite, display_path: &str) -> Result<(), String> {
    if let Some(dialects) = suite.dialects.as_ref() {
        if dialects.is_empty() {
            return Err(format!(
                "Suite {} has an empty dialects list.",
                display_path
            ));
        }

        let mut seen = HashSet::with_capacity(dialects.len());
        for dialect in dialects {
            if !seen.insert(*dialect) {
                return Err(format!(
                    "Suite {} has duplicated dialect '{}' in dialects.",
                    display_path, dialect
                ));
            }
        }
    }

    for (index, migration) in suite.migrations.iter().enumerate() {
        match migration {
            YamlMigration::Sql(sql) => {
                if sql.trim().is_empty() {
                    return Err(format!(
                        "Suite {} migration #{} has empty SQL.",
                        display_path,
                        index + 1
                    ));
                }
            },
            YamlMigration::Case(case) => {
                if let Some(name) = case.name.as_deref() {
                    if name.trim().is_empty() {
                        return Err(format!(
                            "Suite {} migration #{} has an empty name.",
                            display_path,
                            index + 1
                        ));
                    }
                }
                if case.sql.trim().is_empty() {
                    return Err(format!(
                        "Suite {} migration #{} has empty SQL.",
                        display_path,
                        index + 1
                    ));
                }
                if let Some(error_code) = case.expected_error_code.as_deref() {
                    if error_code.trim().is_empty() {
                        return Err(format!(
                            "Suite {} migration #{} has an empty expected_error_code.",
                            display_path,
                            index + 1
                        ));
                    }
                }
            },
        }
    }

    let mut names = HashSet::with_capacity(suite.queries.len());
    for query in &suite.queries {
        if query.name.trim().is_empty() {
            return Err(format!("Suite {} has an empty query name.", display_path));
        }
        if !names.insert(query.name.as_str()) {
            return Err(format!(
                "Suite {} has duplicated query name '{}'.",
                display_path, query.name
            ));
        }
        if query.sql.trim().is_empty() {
            return Err(format!(
                "Suite {} query '{}' has empty SQL.",
                display_path, query.name
            ));
        }
        if let Some(reason) = query.tdd_reason.as_deref() {
            if reason.trim().is_empty() {
                return Err(format!(
                    "Suite {} query '{}' has an empty tdd_reason.",
                    display_path, query.name
                ));
            }
        }
        if let Some(error_code) = query.expected_error_code.as_deref() {
            if error_code.trim().is_empty() {
                return Err(format!(
                    "Suite {} query '{}' has an empty expected_error_code.",
                    display_path, query.name
                ));
            }
        }
        if query.expected_error_code.is_some() && !query.expected.is_empty() {
            return Err(format!(
                "Suite {} query '{}' cannot define both expected and expected_error_code.",
                display_path, query.name
            ));
        }
        for expected_col in &query.expected {
            if expected_col.name.trim().is_empty() {
                return Err(format!(
                    "Suite {} query '{}' has an expected column with empty name.",
                    display_path, query.name
                ));
            }
        }

        if query.expected_error_code.is_none()
            && !query.expected.is_empty()
            && query.cardinality.is_none()
        {
            return Err(format!(
                "Suite {} query '{}' must set cardinality for success assertions.",
                display_path, query.name
            ));
        }
        if query.expected_error_code.is_some() && query.cardinality.is_some() {
            return Err(format!(
                "Suite {} query '{}' cannot define cardinality with expected_error_code.",
                display_path, query.name
            ));
        }
    }

    Ok(())
}

async fn run_test_file(
    path: &Path,
    specs_dir: &Path,
    run_tdd: bool,
    stats: &mut RunStats,
) -> Result<(), String> {
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
    validate_suite(&suite, &display_path)?;

    let dialects = suite
        .dialects
        .clone()
        .unwrap_or_else(|| all_dialects().to_vec());

    for dialect in dialects {
        stats.dialect_runs += 1;
        run_test_file_for_dialect(dialect, &suite, &display_path, run_tdd, stats).await;
    }

    Ok(())
}

async fn run_test_file_for_dialect(
    dialect: Dialect,
    suite: &YamlTestSuite,
    display_path: &str,
    run_tdd: bool,
    stats: &mut RunStats,
) {
    println!("  Dialect: {}", dialect);

    let mut static_analyzer = StaticAnalyzer::new(dialect);
    let mut db_analyzer = new_database_analyzer(dialect).await.unwrap_or_else(|e| {
        panic!(
            "Failed to create database analyzer for {}: {}",
            display_path, e
        )
    });

    for (index, migration) in suite.migrations.iter().enumerate() {
        let migration_name = migration.name(index + 1);
        let sql = migration.sql().trim();
        if sql.is_empty() {
            continue;
        }

        let static_result = static_analyzer.execute(sql).await;
        let db_result = db_analyzer.execute(sql).await;
        if let Some(expected_error_code) = migration.expected_error_code() {
            assert_error_code_for_migration(
                display_path,
                dialect,
                &migration_name,
                sql,
                expected_error_code,
                db_result,
                static_result,
            );
        } else {
            assert_same_status_for_migration(display_path, sql, db_result, static_result);
        }
    }

    verify_schema_equivalence(display_path, &mut db_analyzer, &mut static_analyzer).await;

    for query in &suite.queries {
        stats.query_cases += 1;
        if let Some(reason) = query.tdd_reason.as_deref() {
            if !run_tdd {
                stats.skipped_tdd_cases += 1;
                println!("  Skipped (TDD): {} ({})", query.name, reason);
                continue;
            }
        }

        println!("  Running test: {}", query.name);
        run_analyze_query_case(
            display_path,
            dialect,
            query,
            &mut db_analyzer,
            &static_analyzer,
        )
        .await;
    }
}

fn assert_same_status_for_migration(
    display_path: &str,
    sql: &str,
    db_result: sqlex_analyzer::Result<()>,
    static_result: sqlex_analyzer::Result<()>,
) {
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

fn assert_error_code_for_migration(
    display_path: &str,
    dialect: Dialect,
    migration_name: &str,
    sql: &str,
    expected_error_code: &str,
    db_result: sqlex_analyzer::Result<()>,
    static_result: sqlex_analyzer::Result<()>,
) {
    match (db_result, static_result) {
        (Err(db_err), Err(static_err)) => {
            let observed_code = extract_error_code(&static_err).unwrap_or_else(|| {
                panic!(
                    "Migration '{}' in {} on {}: failed to extract static error code from '{}'",
                    migration_name, display_path, dialect, static_err
                )
            });
            assert_eq!(
                observed_code, expected_error_code,
                "Migration '{}' in {} on {}: expected static error code {}, got {}",
                migration_name, display_path, dialect, expected_error_code, observed_code
            );
            println!(
                "  Migration result: db=ERROR, static=ERROR (code={})",
                observed_code
            );
            println!("    SQL: {}", sql);
            println!("    DB error: {}", db_err);
            println!("    Static error: {}", static_err);
        },
        (Err(db_err), Ok(_)) => {
            panic!(
                "Migration '{}' in {} on {} expected error code {}, but static analyzer succeeded while db failed: {}",
                migration_name, display_path, dialect, expected_error_code, db_err
            );
        },
        (Ok(_), Err(static_err)) => {
            panic!(
                "Migration '{}' in {} on {} expected error code {}, but database analyzer succeeded while static failed: {}",
                migration_name, display_path, dialect, expected_error_code, static_err
            );
        },
        (Ok(_), Ok(_)) => {
            panic!(
                "Migration '{}' in {} on {} expected error code {}, but both analyzers succeeded.",
                migration_name, display_path, dialect, expected_error_code
            );
        },
    }
}

async fn verify_schema_equivalence(
    display_path: &str,
    db_analyzer: &mut Box<dyn Analyzer>,
    static_analyzer: &mut StaticAnalyzer,
) {
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
}

async fn run_analyze_query_case(
    display_path: &str,
    dialect: Dialect,
    query: &YamlQuery,
    db_analyzer: &mut Box<dyn Analyzer>,
    static_analyzer: &StaticAnalyzer,
) {
    let db_result = db_analyzer.analyze(&query.sql).await;
    let static_result = static_analyzer.analyze(&query.sql).await;

    if let Some(expected_error_code) = query.expected_error_code.as_deref() {
        match (db_result, static_result) {
            (Err(db_err), Err(static_err)) => {
                let observed_code = extract_error_code(&static_err).unwrap_or_else(|| {
                    panic!(
                        "Test '{}' in {} on {}: failed to extract static error code from '{}'",
                        query.name, display_path, dialect, static_err
                    )
                });
                assert_eq!(
                    observed_code, expected_error_code,
                    "Test '{}' in {} on {}: expected static error code {}, got {}",
                    query.name, display_path, dialect, expected_error_code, observed_code
                );
                println!("  Result: db=ERROR, static=ERROR (code={})", observed_code);
                println!("    DB error: {}", db_err);
                println!("    Static error: {}", static_err);
            },
            (Err(db_err), Ok(_)) => {
                panic!(
                    "Test '{}' in {} on {} expected error code {}, but static analyzer succeeded while db failed: {}",
                    query.name, display_path, dialect, expected_error_code, db_err
                );
            },
            (Ok(_), Err(static_err)) => {
                panic!(
                    "Test '{}' in {} on {} expected error code {}, but database analyzer succeeded while static failed: {}",
                    query.name, display_path, dialect, expected_error_code, static_err
                );
            },
            (Ok(_), Ok(_)) => {
                panic!(
                    "Test '{}' in {} on {} expected error code {}, but both analyzers succeeded.",
                    query.name, display_path, dialect, expected_error_code
                );
            },
        }
        return;
    }

    if query.expected.is_empty() {
        match (db_result, static_result) {
            (Err(db_err), Err(static_err)) => {
                println!("  Result: db=ERROR, static=ERROR (match)");
                println!("    DB error: {}", db_err);
                println!("    Static error: {}", static_err);
            },
            (Ok(_), Ok(_)) => {
                println!("  Result: db=OK, static=OK (status-only check)");
            },
            (Err(db_err), Ok(_)) => {
                panic!(
                    "Test '{}' in {} on {}: database analyzer failed, but static analyzer succeeded ({})",
                    query.name, display_path, dialect, db_err
                );
            },
            (Ok(_), Err(static_err)) => {
                panic!(
                    "Test '{}' in {} on {}: database analyzer succeeded, but static analyzer failed ({})",
                    query.name, display_path, dialect, static_err
                );
            },
        }
        return;
    }

    let expected_cardinality = query.cardinality.unwrap_or_else(|| {
        panic!(
            "Test '{}' in {} on {}: missing cardinality for success assertion.",
            query.name, display_path, dialect
        )
    });

    match (db_result, static_result) {
        (Err(db_err), Err(static_err)) => {
            panic!(
                "Test '{}' in {} on {} expected success assertion, but both analyzers failed. db='{}', static='{}'",
                query.name, display_path, dialect, db_err, static_err
            );
        },
        (Err(db_err), Ok(_)) => {
            panic!(
                "Test '{}' in {} on {} expected success assertion, but database analyzer failed: {}",
                query.name, display_path, dialect, db_err
            );
        },
        (Ok(_), Err(static_err)) => {
            panic!(
                "Test '{}' in {} on {} expected success assertion, but static analyzer failed: {}",
                query.name, display_path, dialect, static_err
            );
        },
        (Ok(db_result), Ok(static_result)) => {
            let db_columns = db_result.columns;
            let static_columns = static_result.columns;

            assert_eq!(
                static_result.cardinality, expected_cardinality,
                "Test '{}' in {} on {}: Cardinality mismatch (Expected: {:?}, Actual: {:?})",
                query.name, display_path, dialect, expected_cardinality, static_result.cardinality
            );

            assert_eq!(
                static_columns.len(),
                db_columns.len(),
                "Test '{}' in {} on {}: output column count mismatch. Expected {}, got {}",
                query.name,
                display_path,
                dialect,
                db_columns.len(),
                static_columns.len()
            );

            assert_eq!(
                query.expected.len(),
                db_columns.len(),
                "Test '{}' in {} on {}: expected nullability count mismatch. Expected {}, got {}",
                query.name,
                display_path,
                dialect,
                db_columns.len(),
                query.expected.len()
            );

            for i in 0..db_columns.len() {
                let db_col = &db_columns[i];
                let static_col = &static_columns[i];
                let expected_col = &query.expected[i];

                assert_eq!(
                    static_col.name, db_col.name,
                    "Test '{}' in {} on {}: Column {} name mismatch (Expected: {}, Actual: {})",
                    query.name, display_path, dialect, i, db_col.name, static_col.name
                );
                assert_eq!(
                    expected_col.name, db_col.name,
                    "Test '{}' in {} on {}: Expected column {} name mismatch (Expected: {}, Actual: {})",
                    query.name, display_path, dialect, i, db_col.name, expected_col.name
                );
                let types_match = if dialect == Dialect::SQLite {
                    sqlite_types_compatible(&static_col.data_type, &db_col.data_type)
                } else {
                    static_col.data_type == db_col.data_type
                };

                assert!(
                    types_match,
                    "Test '{}' in {} on {}: Column {} type mismatch (Expected: {:?}, Actual: {:?})",
                    query.name, display_path, dialect, i, db_col.data_type, static_col.data_type
                );
                assert_eq!(
                    static_col.nullability,
                    expected_col.nullability,
                    "Test '{}' in {} on {}: Column {} nullability mismatch (Expected: {}, Actual: {})",
                    query.name,
                    display_path,
                    dialect,
                    i,
                    expected_col.nullability,
                    static_col.nullability
                );
            }
        },
    }
}
