//! Nullability Inference Tests
//!
//! Tests for expression nullability analysis

use sqlex_common::DataType;
use sqlex_static_analyzer::{ColumnDef, Dialect, QueryAnalyzer, Schema, TableDef};

fn setup_schema() -> Schema {
    let mut schema = Schema::new(Dialect::PostgreSQL);

    let users = TableDef {
        name: "users".to_string(),
        columns: vec![
            ColumnDef::new("id", DataType::Int).not_null(),
            ColumnDef::new("name", DataType::Text).not_null(),
            ColumnDef::new("email", DataType::Text), // nullable
            ColumnDef::new("age", DataType::Int),    // nullable
            ColumnDef::new("score", DataType::Double), // nullable
        ],
        primary_key: Some(vec!["id".to_string()]),
        unique_constraints: vec![],
        foreign_keys: vec![],
    };
    schema.add_table(users);
    schema
}

// =============================================================================
// Column Reference Nullability
// =============================================================================

#[test]
fn test_not_null_column_is_not_nullable() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    let result = analyzer.analyze("SELECT id FROM users").unwrap();

    assert!(!result.columns[0].nullability);
}

#[test]
fn test_nullable_column_is_nullable() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    let result = analyzer.analyze("SELECT email FROM users").unwrap();

    assert!(result.columns[0].nullability);
}

#[test]
fn test_mixed_nullability() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    let result = analyzer
        .analyze("SELECT id, name, email FROM users")
        .unwrap();

    assert!(!result.columns[0].nullability); // id: NOT NULL
    assert!(!result.columns[1].nullability); // name: NOT NULL
    assert!(result.columns[2].nullability); // email: nullable
}

// =============================================================================
// Binary Operator Nullability
// =============================================================================

#[test]
fn test_binary_op_with_nullable() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    // age is nullable, so age + 1 is nullable
    let result = analyzer.analyze("SELECT age + 1 FROM users").unwrap();
    assert!(result.columns[0].nullability);
}

#[test]
fn test_binary_op_not_null() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    // id is NOT NULL, so id + 1 is NOT NULL
    let result = analyzer.analyze("SELECT id + 1 FROM users").unwrap();
    assert!(!result.columns[0].nullability);
}

#[test]
fn test_binary_op_both_nullable() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    // Both age and score are nullable
    let result = analyzer.analyze("SELECT age + score FROM users").unwrap();
    assert!(result.columns[0].nullability);
}

// =============================================================================
// COALESCE Nullability
// =============================================================================

#[test]
fn test_coalesce_with_not_null() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    // COALESCE(nullable, not_null_literal) => not nullable
    let result = analyzer
        .analyze("SELECT COALESCE(age, 0) FROM users")
        .unwrap();
    assert!(!result.columns[0].nullability);
}

#[test]
fn test_coalesce_all_nullable() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    // COALESCE(nullable, nullable) => nullable
    let result = analyzer
        .analyze("SELECT COALESCE(age, score) FROM users")
        .unwrap();
    assert!(result.columns[0].nullability);
}

#[test]
fn test_coalesce_multiple_args() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    // COALESCE(nullable, nullable, not_null) => not nullable
    let result = analyzer
        .analyze("SELECT COALESCE(age, score, 0) FROM users")
        .unwrap();
    assert!(!result.columns[0].nullability);
}

// =============================================================================
// NULLIF Nullability
// =============================================================================

#[test]
fn test_nullif_always_nullable() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    // NULLIF can always return NULL
    let result = analyzer.analyze("SELECT NULLIF(id, 0) FROM users").unwrap();
    assert!(result.columns[0].nullability);
}

// =============================================================================
// CASE Expression Nullability
// =============================================================================

#[test]
fn test_case_with_else_all_not_null() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    // CASE with ELSE and all branches NOT NULL => not nullable
    let result = analyzer
        .analyze("SELECT CASE WHEN id > 0 THEN 'positive' ELSE 'zero' END FROM users")
        .unwrap();
    assert!(!result.columns[0].nullability);
}

#[test]
fn test_case_without_else() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    // CASE without ELSE => nullable (implicit NULL)
    let result = analyzer
        .analyze("SELECT CASE WHEN id > 0 THEN 'positive' END FROM users")
        .unwrap();
    assert!(result.columns[0].nullability);
}

#[test]
fn test_case_with_nullable_branch() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    // CASE with nullable branch => nullable
    let result = analyzer
        .analyze("SELECT CASE WHEN id > 0 THEN age ELSE 0 END FROM users")
        .unwrap();
    assert!(result.columns[0].nullability);
}

// =============================================================================
// Subquery Nullability
// =============================================================================

#[test]
fn test_scalar_subquery_nullable() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    // Scalar subquery can return no rows => nullable
    let result = analyzer
        .analyze("SELECT (SELECT id FROM users WHERE id = 999) FROM users")
        .unwrap();
    assert!(result.columns[0].nullability);
}

// =============================================================================
// Aggregate Function Nullability
// =============================================================================

#[test]
fn test_count_star_not_nullable() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    let result = analyzer.analyze("SELECT COUNT(*) FROM users").unwrap();

    assert!(!result.columns[0].nullability); // COUNT(*) never returns NULL
}

#[test]
fn test_count_column_not_nullable() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    let result = analyzer.analyze("SELECT COUNT(age) FROM users").unwrap();

    assert!(!result.columns[0].nullability); // COUNT(col) never returns NULL
}

#[test]
fn test_sum_nullable() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    let result = analyzer.analyze("SELECT SUM(age) FROM users").unwrap();

    assert!(result.columns[0].nullability); // SUM returns NULL for empty set
}

#[test]
fn test_avg_nullable() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    let result = analyzer.analyze("SELECT AVG(age) FROM users").unwrap();

    assert!(result.columns[0].nullability);
}

#[test]
fn test_min_max_nullable() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    let result = analyzer
        .analyze("SELECT MIN(age), MAX(age) FROM users")
        .unwrap();

    assert!(result.columns[0].nullability);
    assert!(result.columns[1].nullability);
}

#[test]
fn test_mixed_aggregates() {
    let schema = setup_schema();
    let mut analyzer = QueryAnalyzer::new(&schema);

    let result = analyzer
        .analyze("SELECT COUNT(*), SUM(age), AVG(age) FROM users")
        .unwrap();

    assert!(!result.columns[0].nullability); // COUNT
    assert!(result.columns[1].nullability); // SUM
    assert!(result.columns[2].nullability); // AVG
}
