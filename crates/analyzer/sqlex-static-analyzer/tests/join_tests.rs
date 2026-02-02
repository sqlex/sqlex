//! JOIN Nullability Tests
//!
//! Tests for JOIN nullability with and without foreign key awareness

use sqlex_common::DataType;
use sqlex_static_analyzer::{ColumnDef, Dialect, ForeignKeyDef, QueryAnalyzer, Schema, TableDef};

// =============================================================================
// Basic JOIN Tests (without FK)
// =============================================================================

fn setup_schema_no_fk() -> Schema {
    let mut schema = Schema::new(Dialect::PostgreSQL);

    let users = TableDef {
        name: "users".to_string(),
        columns: vec![
            ColumnDef::new("id", DataType::Int).not_null(),
            ColumnDef::new("name", DataType::Text).not_null(),
        ],
        primary_key: Some(vec!["id".to_string()]),
        unique_constraints: vec![],
        foreign_keys: vec![],
    };
    schema.add_table(users);

    // orders WITHOUT foreign key
    let orders = TableDef {
        name: "orders".to_string(),
        columns: vec![
            ColumnDef::new("id", DataType::Int).not_null(),
            ColumnDef::new("user_id", DataType::Int), // nullable, no FK
            ColumnDef::new("amount", DataType::Decimal).not_null(),
        ],
        primary_key: Some(vec!["id".to_string()]),
        unique_constraints: vec![],
        foreign_keys: vec![], // No FK!
    };
    schema.add_table(orders);

    schema
}

#[test]
fn test_inner_join_preserves_nullability() {
    let schema = setup_schema_no_fk();
    let mut analyzer = QueryAnalyzer::new(&schema);

    let result = analyzer
        .analyze(
            "SELECT u.id, u.name, o.amount 
             FROM users u 
             INNER JOIN orders o ON u.id = o.user_id",
        )
        .unwrap();

    assert!(!result.columns[0].nullability); // u.id: NOT NULL
    assert!(!result.columns[1].nullability); // u.name: NOT NULL
    assert!(!result.columns[2].nullability); // o.amount: NOT NULL
}

#[test]
fn test_left_join_right_columns_nullable() {
    let schema = setup_schema_no_fk();
    let mut analyzer = QueryAnalyzer::new(&schema);

    // Without FK, right table columns become nullable in LEFT JOIN
    let result = analyzer
        .analyze(
            "SELECT u.id, o.id, o.amount 
             FROM users u 
             LEFT JOIN orders o ON u.id = o.user_id",
        )
        .unwrap();

    assert!(!result.columns[0].nullability); // u.id: left table keeps nullability
    assert!(result.columns[1].nullability); // o.id: forced nullable (no FK)
    assert!(result.columns[2].nullability); // o.amount: forced nullable (no FK)
}

#[test]
fn test_right_join_left_columns_nullable() {
    let schema = setup_schema_no_fk();
    let mut analyzer = QueryAnalyzer::new(&schema);

    let result = analyzer
        .analyze(
            "SELECT u.id, u.name, o.amount 
             FROM users u 
             RIGHT JOIN orders o ON u.id = o.user_id",
        )
        .unwrap();

    assert!(result.columns[0].nullability); // u.id: forced nullable
    assert!(result.columns[1].nullability); // u.name: forced nullable
    assert!(!result.columns[2].nullability); // o.amount: right table keeps nullability
}

#[test]
fn test_full_join_both_sides_nullable() {
    let schema = setup_schema_no_fk();
    let mut analyzer = QueryAnalyzer::new(&schema);

    let result = analyzer
        .analyze(
            "SELECT u.id, o.id 
             FROM users u 
             FULL OUTER JOIN orders o ON u.id = o.user_id",
        )
        .unwrap();

    assert!(result.columns[0].nullability); // u.id: forced nullable
    assert!(result.columns[1].nullability); // o.id: forced nullable
}

#[test]
fn test_cross_join_preserves_nullability() {
    let schema = setup_schema_no_fk();
    let mut analyzer = QueryAnalyzer::new(&schema);

    let result = analyzer
        .analyze(
            "SELECT u.id, o.amount 
             FROM users u 
             CROSS JOIN orders o",
        )
        .unwrap();

    assert!(!result.columns[0].nullability); // preserves original
    assert!(!result.columns[1].nullability); // preserves original
}

// =============================================================================
// FK-Aware JOIN Tests
// =============================================================================

fn setup_schema_with_fk() -> Schema {
    let mut schema = Schema::new(Dialect::PostgreSQL);

    let users = TableDef {
        name: "users".to_string(),
        columns: vec![
            ColumnDef::new("id", DataType::Int).not_null(),
            ColumnDef::new("name", DataType::Text).not_null(),
        ],
        primary_key: Some(vec!["id".to_string()]),
        unique_constraints: vec![],
        foreign_keys: vec![],
    };
    schema.add_table(users);

    // orders WITH foreign key and NOT NULL user_id
    let orders = TableDef {
        name: "orders".to_string(),
        columns: vec![
            ColumnDef::new("id", DataType::Int).not_null(),
            ColumnDef::new("user_id", DataType::Int).not_null(), // NOT NULL + FK
            ColumnDef::new("amount", DataType::Decimal).not_null(),
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

#[test]
fn test_left_join_fk_preserves_right_nullability() {
    let schema = setup_schema_with_fk();
    let mut analyzer = QueryAnalyzer::new(&schema);

    // orders LEFT JOIN users: FK guarantees every order has a matching user
    // because orders.user_id REFERENCES users.id AND is NOT NULL
    let result = analyzer
        .analyze(
            "SELECT o.id, u.id, u.name 
             FROM orders o 
             LEFT JOIN users u ON o.user_id = u.id",
        )
        .unwrap();

    assert!(!result.columns[0].nullability); // o.id: NOT NULL
    assert!(!result.columns[1].nullability); // u.id: FK guarantees match!
    assert!(!result.columns[2].nullability); // u.name: FK guarantees match!
}

#[test]
fn test_left_join_no_fk_forces_nullable() {
    let schema = setup_schema_with_fk();
    let mut analyzer = QueryAnalyzer::new(&schema);

    // users LEFT JOIN orders: No FK from users to orders
    // A user may have no orders
    let result = analyzer
        .analyze(
            "SELECT u.id, o.id, o.amount 
             FROM users u 
             LEFT JOIN orders o ON u.id = o.user_id",
        )
        .unwrap();

    assert!(!result.columns[0].nullability); // u.id: left table
    assert!(result.columns[1].nullability); // o.id: nullable (no FK guarantee)
    assert!(result.columns[2].nullability); // o.amount: nullable
}

#[test]
fn test_right_join_fk_preserves_left_nullability() {
    let schema = setup_schema_with_fk();
    let mut analyzer = QueryAnalyzer::new(&schema);

    // users RIGHT JOIN orders: Same as orders LEFT JOIN users
    // FK guarantees every order has a user
    let result = analyzer
        .analyze(
            "SELECT u.id, u.name, o.id 
             FROM users u 
             RIGHT JOIN orders o ON u.id = o.user_id",
        )
        .unwrap();

    assert!(!result.columns[0].nullability); // u.id: FK guarantees match!
    assert!(!result.columns[1].nullability); // u.name: FK guarantees match!
    assert!(!result.columns[2].nullability); // o.id: right table
}

// =============================================================================
// Multiple JOIN Tests
// =============================================================================

fn setup_schema_with_products() -> Schema {
    let mut schema = Schema::new(Dialect::PostgreSQL);

    let users = TableDef {
        name: "users".to_string(),
        columns: vec![
            ColumnDef::new("id", DataType::Int).not_null(),
            ColumnDef::new("name", DataType::Text).not_null(),
        ],
        primary_key: Some(vec!["id".to_string()]),
        unique_constraints: vec![],
        foreign_keys: vec![],
    };
    schema.add_table(users);

    let products = TableDef {
        name: "products".to_string(),
        columns: vec![
            ColumnDef::new("id", DataType::Int).not_null(),
            ColumnDef::new("name", DataType::Text).not_null(),
        ],
        primary_key: Some(vec!["id".to_string()]),
        unique_constraints: vec![],
        foreign_keys: vec![],
    };
    schema.add_table(products);

    let orders = TableDef {
        name: "orders".to_string(),
        columns: vec![
            ColumnDef::new("id", DataType::Int).not_null(),
            ColumnDef::new("user_id", DataType::Int).not_null(),
            ColumnDef::new("product_id", DataType::Int).not_null(),
        ],
        primary_key: Some(vec!["id".to_string()]),
        unique_constraints: vec![],
        foreign_keys: vec![
            ForeignKeyDef::new(vec!["user_id".to_string()], "users", vec!["id".to_string()]),
            ForeignKeyDef::new(
                vec!["product_id".to_string()],
                "products",
                vec!["id".to_string()],
            ),
        ],
    };
    schema.add_table(orders);

    schema
}

#[test]
fn test_multiple_left_joins_with_fk() {
    let schema = setup_schema_with_products();
    let mut analyzer = QueryAnalyzer::new(&schema);

    // orders LEFT JOIN users LEFT JOIN products
    // Both FKs are NOT NULL, so all should be NOT NULL
    let result = analyzer
        .analyze(
            "SELECT o.id, u.name, p.name
             FROM orders o
             LEFT JOIN users u ON o.user_id = u.id
             LEFT JOIN products p ON o.product_id = p.id",
        )
        .unwrap();

    assert!(!result.columns[0].nullability); // o.id
    assert!(!result.columns[1].nullability); // u.name: FK guarantee
    assert!(!result.columns[2].nullability); // p.name: FK guarantee
}

#[test]
fn test_chained_joins() {
    let schema = setup_schema_with_products();
    let mut analyzer = QueryAnalyzer::new(&schema);

    let result = analyzer
        .analyze(
            "SELECT u.name
             FROM users u
             INNER JOIN orders o ON u.id = o.user_id
             INNER JOIN products p ON o.product_id = p.id",
        )
        .unwrap();

    assert!(!result.columns[0].nullability);
}

// =============================================================================
// JOIN with USING clause
// =============================================================================

#[test]
fn test_join_using_clause() {
    let schema = setup_schema_no_fk();
    let mut analyzer = QueryAnalyzer::new(&schema);

    // This requires both tables to have a column with the same name
    // We'll test with a modified setup
    let result = analyzer
        .analyze(
            "SELECT u.id, u.name
             FROM users u
             INNER JOIN (SELECT id, amount FROM orders) o USING (id)",
        )
        .unwrap();

    assert_eq!(result.columns.len(), 2);
}

// =============================================================================
// Self JOIN Tests
// =============================================================================

#[test]
fn test_self_join() {
    let mut schema = Schema::new(Dialect::PostgreSQL);

    let employees = TableDef {
        name: "employees".to_string(),
        columns: vec![
            ColumnDef::new("id", DataType::Int).not_null(),
            ColumnDef::new("name", DataType::Text).not_null(),
            ColumnDef::new("manager_id", DataType::Int), // nullable
        ],
        primary_key: Some(vec!["id".to_string()]),
        unique_constraints: vec![],
        foreign_keys: vec![],
    };
    schema.add_table(employees);

    let mut analyzer = QueryAnalyzer::new(&schema);

    let result = analyzer
        .analyze(
            "SELECT e.name, m.name AS manager_name
             FROM employees e
             LEFT JOIN employees m ON e.manager_id = m.id",
        )
        .unwrap();

    assert!(!result.columns[0].nullability); // e.name
    assert!(result.columns[1].nullability); // m.name (no FK guarantee)
}
