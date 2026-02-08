//! DDL Parsing Tests
//!
//! Tests for CREATE TABLE, ALTER TABLE, DROP TABLE parsing

use sqlex_common::{dialect::Dialect, types::DataType};
use sqlex_static_analyzer::catalog::Catalog;

#[test]
fn test_create_table_simple() {
    let mut schema = Catalog::new(Dialect::Postgres);
    schema
        .apply_ddl("CREATE TABLE users (id INT, name TEXT)")
        .unwrap();

    let table = schema.get_table("users").expect("table should exist");
    assert_eq!(table.name, "users");
    assert_eq!(table.columns.len(), 2);
    assert_eq!(table.columns[0].name, "id");
    assert_eq!(table.columns[1].name, "name");
}

#[test]
fn test_create_table_with_not_null() {
    let mut schema = Catalog::new(Dialect::Postgres);
    schema
        .apply_ddl("CREATE TABLE users (id INT NOT NULL, name TEXT)")
        .unwrap();

    let table = schema.get_table("users").unwrap();
    assert!(!table.columns[0].nullable); // id is NOT NULL
    assert!(table.columns[1].nullable); // name is nullable
}

#[test]
fn test_create_table_with_primary_key_inline() {
    let mut schema = Catalog::new(Dialect::Postgres);
    schema
        .apply_ddl("CREATE TABLE users (id INT PRIMARY KEY, name TEXT)")
        .unwrap();

    let table = schema.get_table("users").unwrap();
    assert!(!table.columns[0].nullable); // PK implies NOT NULL
    assert_eq!(table.primary_key, Some(vec!["id".to_string()]));
}

#[test]
fn test_create_table_with_primary_key_constraint() {
    let mut schema = Catalog::new(Dialect::Postgres);
    schema
        .apply_ddl(
            "CREATE TABLE users (
                id INT,
                name TEXT,
                PRIMARY KEY (id)
            )",
        )
        .unwrap();

    let table = schema.get_table("users").unwrap();
    assert!(!table.columns[0].nullable);
    assert_eq!(table.primary_key, Some(vec!["id".to_string()]));
}

#[test]
fn test_create_table_with_composite_primary_key() {
    let mut schema = Catalog::new(Dialect::Postgres);
    schema
        .apply_ddl(
            "CREATE TABLE order_items (
                order_id INT,
                item_id INT,
                quantity INT,
                PRIMARY KEY (order_id, item_id)
            )",
        )
        .unwrap();

    let table = schema.get_table("order_items").unwrap();
    assert!(!table.columns[0].nullable);
    assert!(!table.columns[1].nullable);
    assert_eq!(
        table.primary_key,
        Some(vec!["order_id".to_string(), "item_id".to_string()])
    );
}

#[test]
fn test_create_table_with_foreign_key_inline() {
    let mut schema = Catalog::new(Dialect::Postgres);
    schema
        .apply_ddl("CREATE TABLE users (id INT PRIMARY KEY)")
        .unwrap();
    schema
        .apply_ddl(
            "CREATE TABLE orders (
                id INT PRIMARY KEY,
                user_id INT NOT NULL REFERENCES users(id)
            )",
        )
        .unwrap();

    let orders = schema.get_table("orders").unwrap();
    assert_eq!(orders.foreign_keys.len(), 1);
    assert_eq!(orders.foreign_keys[0].columns, vec!["user_id"]);
    assert_eq!(orders.foreign_keys[0].ref_table, "users");
    assert_eq!(orders.foreign_keys[0].ref_columns, vec!["id"]);
}

#[test]
fn test_create_table_with_foreign_key_constraint() {
    let mut schema = Catalog::new(Dialect::Postgres);
    schema
        .apply_ddl("CREATE TABLE users (id INT PRIMARY KEY)")
        .unwrap();
    schema
        .apply_ddl(
            "CREATE TABLE orders (
                id INT PRIMARY KEY,
                user_id INT NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users(id)
            )",
        )
        .unwrap();

    let orders = schema.get_table("orders").unwrap();
    assert_eq!(orders.foreign_keys.len(), 1);
}

#[test]
fn test_create_table_with_unique_constraint() {
    let mut schema = Catalog::new(Dialect::Postgres);
    schema
        .apply_ddl(
            "CREATE TABLE users (
                id INT PRIMARY KEY,
                email TEXT UNIQUE
            )",
        )
        .unwrap();

    let table = schema.get_table("users").unwrap();
    assert!(
        table
            .unique_constraints
            .contains(&vec!["email".to_string()])
    );
}

#[test]
fn test_create_table_with_default() {
    let mut schema = Catalog::new(Dialect::Postgres);
    schema
        .apply_ddl(
            "CREATE TABLE users (
                id INT PRIMARY KEY,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .unwrap();

    let table = schema.get_table("users").unwrap();
    assert!(table.columns[1].default.is_some());
}

#[test]
fn test_alter_table_add_column() {
    let mut schema = Catalog::new(Dialect::Postgres);
    schema
        .apply_ddl("CREATE TABLE users (id INT PRIMARY KEY)")
        .unwrap();
    schema
        .apply_ddl("ALTER TABLE users ADD COLUMN email TEXT")
        .unwrap();

    let table = schema.get_table("users").unwrap();
    assert_eq!(table.columns.len(), 2);
    assert_eq!(table.columns[1].name, "email");
}

#[test]
fn test_alter_table_add_foreign_key() {
    let mut schema = Catalog::new(Dialect::Postgres);
    schema
        .apply_ddl("CREATE TABLE users (id INT PRIMARY KEY)")
        .unwrap();
    schema
        .apply_ddl("CREATE TABLE orders (id INT PRIMARY KEY, user_id INT NOT NULL)")
        .unwrap();
    schema
        .apply_ddl(
            "ALTER TABLE orders ADD CONSTRAINT fk_user 
            FOREIGN KEY (user_id) REFERENCES users(id)",
        )
        .unwrap();

    let orders = schema.get_table("orders").unwrap();
    assert_eq!(orders.foreign_keys.len(), 1);
}

#[test]
fn test_drop_table() {
    let mut schema = Catalog::new(Dialect::Postgres);
    schema
        .apply_ddl("CREATE TABLE users (id INT PRIMARY KEY)")
        .unwrap();
    schema.apply_ddl("DROP TABLE users").unwrap();

    assert!(schema.get_table("users").is_none());
}

#[test]
fn test_multiple_statements() {
    let mut schema = Catalog::new(Dialect::Postgres);
    schema
        .apply_ddl(
            "
        CREATE TABLE users (id INT PRIMARY KEY);
        CREATE TABLE orders (id INT PRIMARY KEY, user_id INT);
    ",
        )
        .unwrap();

    assert!(schema.get_table("users").is_some());
    assert!(schema.get_table("orders").is_some());
}

// Dialect-specific tests

#[test]
fn test_mysql_auto_increment() {
    let mut schema = Catalog::new(Dialect::MySQL);
    schema
        .apply_ddl(
            "CREATE TABLE users (
                id INT AUTO_INCREMENT PRIMARY KEY,
                name VARCHAR(255)
            )",
        )
        .unwrap();

    let table = schema.get_table("users").unwrap();
    assert!(!table.columns[0].nullable);
}

#[test]
fn test_sqlite_integer_primary_key() {
    let mut schema = Catalog::new(Dialect::SQLite);
    schema
        .apply_ddl("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
        .unwrap();

    let table = schema.get_table("users").unwrap();
    assert!(!table.columns[0].nullable);
}

// Type mapping tests

#[test]
fn test_data_type_mapping_int() {
    let mut schema = Catalog::new(Dialect::Postgres);
    schema
        .apply_ddl("CREATE TABLE t (a INT, b INTEGER, c SMALLINT, d BIGINT)")
        .unwrap();

    let table = schema.get_table("t").unwrap();
    assert_eq!(table.columns[0].data_type, DataType::Int(false));
    assert_eq!(table.columns[1].data_type, DataType::Int(false));
    assert_eq!(table.columns[2].data_type, DataType::SmallInt(false));
    assert_eq!(table.columns[3].data_type, DataType::BigInt(false));
}

#[test]
fn test_data_type_mapping_float() {
    let mut schema = Catalog::new(Dialect::Postgres);
    schema
        .apply_ddl("CREATE TABLE t (a FLOAT, b DOUBLE PRECISION, c DECIMAL, d NUMERIC)")
        .unwrap();

    let table = schema.get_table("t").unwrap();
    assert_eq!(table.columns[0].data_type, DataType::Float);
    assert_eq!(table.columns[1].data_type, DataType::Double);
    assert_eq!(table.columns[2].data_type, DataType::Decimal);
    assert_eq!(table.columns[3].data_type, DataType::Decimal);
}

#[test]
fn test_data_type_mapping_text() {
    let mut schema = Catalog::new(Dialect::Postgres);
    schema
        .apply_ddl("CREATE TABLE t (a TEXT, b VARCHAR(255), c CHAR(10))")
        .unwrap();

    let table = schema.get_table("t").unwrap();
    assert_eq!(table.columns[0].data_type, DataType::Text);
    assert_eq!(table.columns[1].data_type, DataType::Varchar);
    assert_eq!(table.columns[2].data_type, DataType::Char);
}

#[test]
fn test_data_type_mapping_time() {
    let mut schema = Catalog::new(Dialect::Postgres);
    schema
        .apply_ddl("CREATE TABLE t (a DATE, b TIME, c TIMESTAMP)")
        .unwrap();

    let table = schema.get_table("t").unwrap();
    assert_eq!(table.columns[0].data_type, DataType::Date);
    assert_eq!(table.columns[1].data_type, DataType::Time);
    assert_eq!(table.columns[2].data_type, DataType::Timestamp);
}
