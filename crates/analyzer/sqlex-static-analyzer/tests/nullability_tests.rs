//! Nullability Inference Tests
//!
//! Tests for expression nullability analysis

mod test_utils;
use sqlex_common::DataType;
use sqlex_static_analyzer::{Catalog, ColumnDef, Dialect, TableDef};
use test_utils::analyze_columns;

fn setup_schema() -> Catalog {
    let mut schema = Catalog::new(Dialect::PostgreSQL);

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
    let result = analyze_columns(&schema, "SELECT id FROM users");

    assert!(!result[0].nullability);
}

#[test]
fn test_nullable_column_is_nullable() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT email FROM users");

    assert!(result[0].nullability);
}

#[test]
fn test_mixed_nullability() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT id, name, email FROM users");

    assert!(!result[0].nullability); // id: NOT NULL
    assert!(!result[1].nullability); // name: NOT NULL
    assert!(result[2].nullability); // email: nullable
}

// =============================================================================
// Binary Operator Nullability
// =============================================================================

#[test]
fn test_binary_op_with_nullable() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT age + 1 FROM users");

    // age is nullable, so age + 1 is nullable
    assert!(result[0].nullability);
}

#[test]
fn test_binary_op_not_null() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT id + 1 FROM users");

    // id is NOT NULL, so id + 1 is NOT NULL
    assert!(!result[0].nullability);
}

#[test]
fn test_binary_op_both_nullable() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT age + score FROM users");

    // Both age and score are nullable
    assert!(result[0].nullability);
}

// =============================================================================
// COALESCE Nullability
// =============================================================================

#[test]
fn test_coalesce_with_not_null() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT COALESCE(age, 0) FROM users");

    // COALESCE(nullable, not_null_literal) => not nullable
    assert!(!result[0].nullability);
}

#[test]
fn test_coalesce_all_nullable() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT COALESCE(age, score) FROM users");

    // COALESCE(nullable, nullable) => nullable
    assert!(result[0].nullability);
}

#[test]
fn test_coalesce_multiple_args() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT COALESCE(age, score, 0) FROM users");

    // COALESCE(nullable, nullable, not_null) => not nullable
    assert!(!result[0].nullability);
}

// =============================================================================
// NULLIF Nullability
// =============================================================================

#[test]
fn test_nullif_always_nullable() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT NULLIF(id, 0) FROM users");

    // NULLIF can always return NULL
    assert!(result[0].nullability);
}

// =============================================================================
// CASE Expression Nullability
// =============================================================================

#[test]
fn test_case_with_else_all_not_null() {
    let schema = setup_schema();
    let result = analyze_columns(
        &schema,
        "SELECT CASE WHEN id > 0 THEN 'positive' ELSE 'zero' END FROM users",
    );

    // CASE with ELSE and all branches NOT NULL => not nullable
    assert!(!result[0].nullability);
}

#[test]
fn test_case_without_else() {
    let schema = setup_schema();
    let result = analyze_columns(
        &schema,
        "SELECT CASE WHEN id > 0 THEN 'positive' END FROM users",
    );

    // CASE without ELSE => nullable (implicit NULL)
    assert!(result[0].nullability);
}

#[test]
fn test_case_with_nullable_branch() {
    let schema = setup_schema();
    let result = analyze_columns(
        &schema,
        "SELECT CASE WHEN id > 0 THEN age ELSE 0 END FROM users",
    );

    // CASE with nullable branch => nullable
    assert!(result[0].nullability);
}

// =============================================================================
// Subquery Nullability
// =============================================================================

#[test]
fn test_scalar_subquery_nullable() {
    let schema = setup_schema();
    let result = analyze_columns(
        &schema,
        "SELECT (SELECT id FROM users WHERE id = 999) FROM users",
    );

    // Scalar subquery can return no rows => nullable
    assert!(result[0].nullability);
}

// =============================================================================
// Aggregate Function Nullability
// =============================================================================

#[test]
fn test_count_star_not_nullable() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT COUNT(*) FROM users");

    assert!(!result[0].nullability); // COUNT(*) never returns NULL
}

#[test]
fn test_count_column_not_nullable() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT COUNT(age) FROM users");

    assert!(!result[0].nullability); // COUNT(col) never returns NULL
}

#[test]
fn test_sum_nullable() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT SUM(age) FROM users");

    assert!(result[0].nullability); // SUM returns NULL for empty set
}

#[test]
fn test_avg_nullable() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT AVG(age) FROM users");

    assert!(result[0].nullability);
}

#[test]
fn test_min_max_nullable() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT MIN(age), MAX(age) FROM users");

    assert!(result[0].nullability);
    assert!(result[1].nullability);
}

#[test]
fn test_mixed_aggregates() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT COUNT(*), SUM(age), AVG(age) FROM users");

    assert!(!result[0].nullability); // COUNT
    assert!(result[1].nullability); // SUM
    assert!(result[2].nullability); // AVG
}
