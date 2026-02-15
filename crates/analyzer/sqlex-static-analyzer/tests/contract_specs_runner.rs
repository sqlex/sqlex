use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use sqlex_analyzer::{Analyzer, AnalyzerError};
use sqlex_common::{
    dialect::Dialect,
    types::{DataType, ResultSet},
};
use sqlex_static_analyzer::StaticAnalyzer;

#[derive(Debug, Deserialize)]
struct CrossDialectSuite {
    dialects: Vec<Dialect>,
    #[serde(default)]
    setup: Vec<String>,
    cases: Vec<CrossDialectCase>,
}

#[derive(Debug, Deserialize)]
struct CrossDialectCase {
    name: String,
    sql: String,
}

#[derive(Debug, Deserialize)]
struct UnsupportedSuite {
    dialects: Vec<Dialect>,
    #[serde(default)]
    setup: Vec<String>,
    cases: Vec<UnsupportedCase>,
}

#[derive(Debug, Deserialize)]
struct UnsupportedCase {
    name: String,
    mode: UnsupportedMode,
    sql: String,
    #[serde(default)]
    setup: Vec<String>,
    expected_codes: HashMap<Dialect, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum UnsupportedMode {
    Analyze,
    Execute,
}

const CONTRACT_SPECS_ROOT: &str = "tests/contract_specs";
const CROSS_DIALECT_DIR: &str = "tests/contract_specs/cross_dialect";
const UNSUPPORTED_DIR: &str = "tests/contract_specs/unsupported";

#[tokio::test]
async fn cross_dialect_contracts() {
    let files = collect_yaml_files(Path::new(CROSS_DIALECT_DIR));
    assert!(
        !files.is_empty(),
        "no cross-dialect contract specs found under {}",
        CROSS_DIALECT_DIR
    );

    for file in files {
        let display_path = display_contract_path(&file);
        let content = fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("failed to read {}: {}", display_path, error));
        let suite: CrossDialectSuite = serde_yaml::from_str(&content).unwrap_or_else(|error| {
            panic!(
                "failed to parse cross-dialect suite {}: {}",
                display_path, error
            )
        });

        assert!(
            suite.dialects.len() >= 2,
            "suite {} must include at least 2 dialects",
            display_path
        );
        assert!(
            !suite.cases.is_empty(),
            "suite {} must include at least one case",
            display_path
        );

        let mut analyzers = Vec::with_capacity(suite.dialects.len());
        for dialect in &suite.dialects {
            let mut analyzer = StaticAnalyzer::new(*dialect);
            execute_setup_statements(&mut analyzer, &suite.setup)
                .await
                .unwrap_or_else(|error| {
                    panic!(
                        "setup failed for suite {} on dialect {}: {}",
                        display_path, dialect, error
                    )
                });
            analyzers.push((*dialect, analyzer));
        }

        for case in &suite.cases {
            let mut results = Vec::with_capacity(analyzers.len());
            for (dialect, analyzer) in &analyzers {
                let result = analyzer.analyze(&case.sql).await.unwrap_or_else(|error| {
                    panic!(
                        "case '{}' in {} failed on dialect {}: {}",
                        case.name, display_path, dialect, error
                    )
                });
                results.push((*dialect, result));
            }

            assert_cross_dialect_consistency(&display_path, case, &results);
        }
    }
}

#[tokio::test]
async fn unsupported_contracts() {
    let files = collect_yaml_files(Path::new(UNSUPPORTED_DIR));
    assert!(
        !files.is_empty(),
        "no unsupported contract specs found under {}",
        UNSUPPORTED_DIR
    );

    for file in files {
        let display_path = display_contract_path(&file);
        let content = fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("failed to read {}: {}", display_path, error));
        let suite: UnsupportedSuite = serde_yaml::from_str(&content).unwrap_or_else(|error| {
            panic!(
                "failed to parse unsupported-diagnostic suite {}: {}",
                display_path, error
            )
        });

        assert!(
            !suite.dialects.is_empty(),
            "suite {} must include at least one dialect",
            display_path
        );
        assert!(
            !suite.cases.is_empty(),
            "suite {} must include at least one case",
            display_path
        );

        for dialect in &suite.dialects {
            for case in &suite.cases {
                let expected_code = case.expected_codes.get(dialect).unwrap_or_else(|| {
                    panic!(
                        "missing expected code for dialect {} in case '{}' ({})",
                        dialect, case.name, display_path
                    )
                });

                let mut analyzer = StaticAnalyzer::new(*dialect);
                execute_setup_statements(&mut analyzer, &suite.setup)
                    .await
                    .unwrap_or_else(|error| {
                        panic!(
                            "suite setup failed for case '{}' in {} on dialect {}: {}",
                            case.name, display_path, dialect, error
                        )
                    });
                execute_setup_statements(&mut analyzer, &case.setup)
                    .await
                    .unwrap_or_else(|error| {
                        panic!(
                            "case setup failed for '{}' in {} on dialect {}: {}",
                            case.name, display_path, dialect, error
                        )
                    });

                let observed_error = match case.mode {
                    UnsupportedMode::Analyze => analyzer
                        .analyze(&case.sql)
                        .await
                        .expect_err("expected analyze to fail"),
                    UnsupportedMode::Execute => analyzer
                        .execute(&case.sql)
                        .await
                        .expect_err("expected execute to fail"),
                };

                let observed_code = extract_error_code(&observed_error).unwrap_or_else(|| {
                    panic!(
                        "failed to extract error code for '{}' in {} on dialect {}: {}",
                        case.name, display_path, dialect, observed_error
                    )
                });

                assert_eq!(
                    observed_code, *expected_code,
                    "case '{}' in {} on dialect {}: expected code {}, got {} (error: {})",
                    case.name, display_path, dialect, expected_code, observed_code, observed_error
                );
            }
        }
    }
}

async fn execute_setup_statements(
    analyzer: &mut StaticAnalyzer,
    statements: &[String],
) -> Result<(), AnalyzerError> {
    for statement in statements {
        analyzer.execute(statement).await?;
    }
    Ok(())
}

fn collect_yaml_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(current) = stack.pop() {
        let entries = match fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "yaml") {
                files.push(path);
            }
        }
    }

    files.sort_by(|left, right| {
        let left_rel = display_contract_path(left);
        let right_rel = display_contract_path(right);
        left_rel.cmp(&right_rel)
    });
    files
}

fn display_contract_path(path: &Path) -> String {
    path.strip_prefix(Path::new(CONTRACT_SPECS_ROOT))
        .unwrap_or(path)
        .display()
        .to_string()
}

fn assert_cross_dialect_consistency(
    suite_path: &str,
    case: &CrossDialectCase,
    results: &[(Dialect, ResultSet)],
) {
    let (baseline_dialect, baseline_result) = &results[0];

    for (dialect, result) in results.iter().skip(1) {
        assert_eq!(
            result.cardinality, baseline_result.cardinality,
            "cardinality mismatch in {} case '{}' between {} and {}",
            suite_path, case.name, baseline_dialect, dialect
        );

        assert_eq!(
            result.columns.len(),
            baseline_result.columns.len(),
            "column count mismatch in {} case '{}' between {} and {}",
            suite_path,
            case.name,
            baseline_dialect,
            dialect
        );

        for (index, (baseline_column, current_column)) in baseline_result
            .columns
            .iter()
            .zip(result.columns.iter())
            .enumerate()
        {
            assert_eq!(
                current_column.name, baseline_column.name,
                "column {} name mismatch in {} case '{}' between {} and {}",
                index, suite_path, case.name, baseline_dialect, dialect
            );
            assert_eq!(
                current_column.nullability, baseline_column.nullability,
                "column {} nullability mismatch in {} case '{}' between {} and {}",
                index, suite_path, case.name, baseline_dialect, dialect
            );
            assert!(
                types_compatible_across_dialects(
                    *baseline_dialect,
                    *dialect,
                    &baseline_column.data_type,
                    &current_column.data_type
                ),
                "column {} type mismatch in {} case '{}' between {} ({:?}) and {} ({:?})",
                index,
                suite_path,
                case.name,
                baseline_dialect,
                baseline_column.data_type,
                dialect,
                current_column.data_type
            );
        }
    }
}

fn types_compatible_across_dialects(
    left_dialect: Dialect,
    right_dialect: Dialect,
    left_type: &DataType,
    right_type: &DataType,
) -> bool {
    if left_type == right_type {
        return true;
    }

    if matches!(left_dialect, Dialect::SQLite) || matches!(right_dialect, Dialect::SQLite) {
        return sqlite_types_compatible(left_type, right_type)
            || sqlite_types_compatible(right_type, left_type);
    }

    false
}

fn sqlite_types_compatible(static_type: &DataType, db_type: &DataType) -> bool {
    if static_type == db_type {
        return true;
    }

    if matches!(db_type, DataType::Custom(value) if value == "null") {
        return true;
    }

    if is_integer_family(static_type) && is_integer_family(db_type) {
        return true;
    }

    matches!(
        (static_type, db_type),
        (DataType::Varchar, DataType::Text) | (DataType::Char, DataType::Text)
    )
}

fn is_integer_family(data_type: &DataType) -> bool {
    matches!(
        data_type,
        DataType::TinyInt
            | DataType::UnsignedTinyInt
            | DataType::SmallInt
            | DataType::UnsignedSmallInt
            | DataType::Int
            | DataType::UnsignedInt
            | DataType::BigInt
            | DataType::UnsignedBigInt
    )
}

fn extract_error_code(error: &AnalyzerError) -> Option<String> {
    let rendered = error.to_string();
    let bracket_start = rendered.find('[')?;
    let after_start = &rendered[(bracket_start + 1)..];
    let bracket_end_relative = after_start.find(']')?;
    let bracket_end = bracket_start + 1 + bracket_end_relative;
    let payload = &rendered[(bracket_start + 1)..bracket_end];
    payload.split(':').nth(1).map(ToString::to_string)
}
