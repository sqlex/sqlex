//! Query Analyzer Tests
//!
//! Tests for SELECT statement analysis

use sqlex_common::DataType;
use sqlex_static_analyzer::{BuildContext, ColumnDef, Dialect, ForeignKeyDef, Schema, TableDef};

fn setup_schema() -> Schema {
    let mut schema = Schema::new(Dialect::PostgreSQL);

    let users = TableDef {
        name: "users".to_string(),
        columns: vec![
            ColumnDef::new("id", DataType::Int).not_null(),
            ColumnDef::new("name", DataType::Text).not_null(),
            ColumnDef::new("email", DataType::Text), // nullable
            ColumnDef::new("age", DataType::Int),    // nullable
        ],
        primary_key: Some(vec!["id".to_string()]),
        unique_constraints: vec![],
        foreign_keys: vec![],
    };
    schema.add_table(users);

    let orders = TableDef {
        name: "orders".to_string(),
        columns: vec![
            ColumnDef::new("id", DataType::Int).not_null(),
            ColumnDef::new("user_id", DataType::Int).not_null(),
            ColumnDef::new("amount", DataType::Decimal),
            ColumnDef::new("status", DataType::Text),
        ],
        primary_key: Some(vec!["id".to_string()]),
        unique_constraints: vec![],
        foreign_keys: vec![ForeignKeyDef::new(
            vec!["user_id".to_string()],
            "users",
            vec!["id".to_string()],
        )],
    };
    schema.add_table(orders);

    schema
}

// =============================================================================
// Basic SELECT Tests
// =============================================================================

#[test]
fn test_select_all_columns() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build("SELECT * FROM users")
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 4);
    assert_eq!(result[0].name, "id");
    assert_eq!(result[1].name, "name");
    assert_eq!(result[2].name, "email");
    assert_eq!(result[3].name, "age");
}

#[test]
fn test_select_specific_columns() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build("SELECT id, name FROM users")
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "id");
    assert_eq!(result[1].name, "name");
}

#[test]
fn test_select_with_alias() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build("SELECT id AS user_id, name AS user_name FROM users")
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result[0].name, "user_id");
    assert_eq!(result[1].name, "user_name");
}

#[test]
fn test_select_with_table_alias() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build("SELECT u.id, u.name FROM users u")
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 2);
}

#[test]
fn test_select_literal_values() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build("SELECT 1, 'hello', 3.14, TRUE FROM users")
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 4);
    // Literals are never null
    assert!(!result[0].nullability);
    assert!(!result[1].nullability);
    assert!(!result[2].nullability);
    assert!(!result[3].nullability);
}

#[test]
fn test_select_null_literal() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build("SELECT NULL FROM users")
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 1);
    assert!(result[0].nullability); // NULL is always nullable
}

#[test]
fn test_select_expression() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build("SELECT age + 1 FROM users")
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 1);
    // age is nullable, so age + 1 is nullable
    assert!(result[0].nullability);
}

#[test]
fn test_select_from_subquery() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build("SELECT sub.id FROM (SELECT id FROM users) sub")
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "id");
}

// =============================================================================
// WHERE Clause Tests
// =============================================================================

#[test]
fn test_select_with_where() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build("SELECT id, name FROM users WHERE id > 0")
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 2);
}

#[test]
fn test_select_with_complex_where() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build("SELECT id FROM users WHERE id > 0 AND name IS NOT NULL")
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 1);
}

// =============================================================================
// ORDER BY / LIMIT Tests
// =============================================================================

#[test]
fn test_select_with_order_by() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build("SELECT id, name FROM users ORDER BY id DESC")
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 2);
}

#[test]
fn test_select_with_limit() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build("SELECT id FROM users LIMIT 10 OFFSET 5")
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 1);
}

// =============================================================================
// DISTINCT Tests
// =============================================================================

#[test]
fn test_select_distinct() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build("SELECT DISTINCT name FROM users")
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 1);
}

// =============================================================================
// Error Cases
// =============================================================================

#[test]
fn test_unknown_table_error() {
    let schema = setup_schema();
    let result = BuildContext::new(&schema).build("SELECT * FROM nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_unknown_column_error() {
    let schema = setup_schema();
    let result = BuildContext::new(&schema).build("SELECT nonexistent FROM users");
    assert!(result.is_err());
}

#[test]
fn test_invalid_sql_error() {
    let schema = setup_schema();
    let result = BuildContext::new(&schema).build("SELECT FROM");
    assert!(result.is_err());
}

#[test]
fn test_empty_sql() {
    let schema = setup_schema();
    let result = BuildContext::new(&schema).build("");
    assert!(result.is_err());
}

#[test]
fn test_multiple_statements_error() {
    let schema = setup_schema();
    let result = BuildContext::new(&schema).build("SELECT id FROM users; SELECT id FROM users");
    assert!(result.is_err());
}

#[test]
fn test_non_select_error() {
    let schema = setup_schema();
    let result = BuildContext::new(&schema).build("INSERT INTO users (id) VALUES (1)");
    assert!(result.is_err());
}
