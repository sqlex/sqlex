//! Advanced SQL Tests
//!
//! Tests for CTE, Set Operations, Window Functions

use sqlex_common::DataType;
use sqlex_static_analyzer::{BuildContext, ColumnDef, Dialect, Schema, TableDef};

fn setup_schema() -> Schema {
    let mut schema = Schema::new(Dialect::PostgreSQL);

    let users = TableDef {
        name: "users".to_string(),
        columns: vec![
            ColumnDef::new("id", DataType::Int).not_null(),
            ColumnDef::new("name", DataType::Text).not_null(),
            ColumnDef::new("manager_id", DataType::Int), // nullable, self-referencing
        ],
        primary_key: Some(vec!["id".to_string()]),
        unique_constraints: vec![],
        foreign_keys: vec![],
    };
    schema.add_table(users);

    let sales = TableDef {
        name: "sales".to_string(),
        columns: vec![
            ColumnDef::new("id", DataType::Int).not_null(),
            ColumnDef::new("region", DataType::Text).not_null(),
            ColumnDef::new("amount", DataType::Decimal).not_null(),
        ],
        primary_key: Some(vec!["id".to_string()]),
        unique_constraints: vec![],
        foreign_keys: vec![],
    };
    schema.add_table(sales);

    let table_a = TableDef {
        name: "table_a".to_string(),
        columns: vec![
            ColumnDef::new("id", DataType::Int).not_null(),
            ColumnDef::new("value", DataType::Text),
        ],
        primary_key: None,
        unique_constraints: vec![],
        foreign_keys: vec![],
    };
    schema.add_table(table_a);

    let table_b = TableDef {
        name: "table_b".to_string(),
        columns: vec![
            ColumnDef::new("id", DataType::Int).not_null(),
            ColumnDef::new("value", DataType::Text).not_null(),
        ],
        primary_key: None,
        unique_constraints: vec![],
        foreign_keys: vec![],
    };
    schema.add_table(table_b);

    schema
}

// =============================================================================
// CTE (WITH clause) Tests
// =============================================================================

#[test]
fn test_simple_cte() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build(
            "WITH active_users AS (
                SELECT id, name FROM users WHERE id > 0
            )
            SELECT id, name FROM active_users",
        )
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 2);
    assert!(!result[0].nullability); // id from CTE
    assert!(!result[1].nullability); // name from CTE
}

#[test]
fn test_cte_with_column_aliases() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build(
            "WITH renamed(user_id, user_name) AS (
                SELECT id, name FROM users
            )
            SELECT user_id, user_name FROM renamed",
        )
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result[0].name, "user_id");
    assert_eq!(result[1].name, "user_name");
}

#[test]
fn test_multiple_ctes() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build(
            "WITH 
                cte1 AS (SELECT id FROM users),
                cte2 AS (SELECT name FROM users)
            SELECT cte1.id, cte2.name FROM cte1, cte2",
        )
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 2);
}

#[test]
fn test_recursive_cte() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build(
            "WITH RECURSIVE hierarchy AS (
                SELECT id, name, manager_id, 1 as level
                FROM users
                WHERE manager_id IS NULL
                UNION ALL
                SELECT u.id, u.name, u.manager_id, h.level + 1
                FROM users u
                JOIN hierarchy h ON u.manager_id = h.id
            )
            SELECT id, name, level FROM hierarchy",
        )
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 3);
}

// =============================================================================
// Set Operations Tests (UNION, INTERSECT, EXCEPT)
// =============================================================================

#[test]
fn test_union_nullability() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build(
            "SELECT id, value FROM table_a
             UNION
             SELECT id, value FROM table_b",
        )
        .unwrap();
    let result = plan.columns(&schema);

    assert!(!result[0].nullability); // id: NOT NULL in both
    assert!(result[1].nullability); // value: nullable in table_a
}

#[test]
fn test_union_all() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build(
            "SELECT id FROM table_a
             UNION ALL
             SELECT id FROM table_b",
        )
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 1);
}

#[test]
fn test_intersect() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build(
            "SELECT id FROM table_a
             INTERSECT
             SELECT id FROM table_b",
        )
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 1);
}

#[test]
fn test_except() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build(
            "SELECT id FROM table_a
             EXCEPT
             SELECT id FROM table_b",
        )
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 1);
}

// =============================================================================
// Window Function Tests
// =============================================================================

#[test]
fn test_row_number_not_nullable() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build("SELECT id, ROW_NUMBER() OVER (ORDER BY id) FROM sales")
        .unwrap();
    let result = plan.columns(&schema);

    assert!(!result[0].nullability);
    assert!(!result[1].nullability); // ROW_NUMBER never null
}

#[test]
fn test_rank_not_nullable() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build("SELECT id, RANK() OVER (ORDER BY amount) FROM sales")
        .unwrap();
    let result = plan.columns(&schema);

    assert!(!result[1].nullability);
}

#[test]
fn test_dense_rank_not_nullable() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build("SELECT id, DENSE_RANK() OVER (ORDER BY amount) FROM sales")
        .unwrap();
    let result = plan.columns(&schema);

    assert!(!result[1].nullability);
}

#[test]
fn test_lead_lag_nullable() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build(
            "SELECT id, 
                    LEAD(amount) OVER (ORDER BY id),
                    LAG(amount) OVER (ORDER BY id)
             FROM sales",
        )
        .unwrap();
    let result = plan.columns(&schema);

    assert!(result[1].nullability); // LEAD
    assert!(result[2].nullability); // LAG
}

#[test]
fn test_sum_over_window() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build("SELECT id, SUM(amount) OVER (PARTITION BY region ORDER BY id) FROM sales")
        .unwrap();
    let result = plan.columns(&schema);

    // Window aggregate - conservatively nullable
    assert!(result[1].nullability);
}

#[test]
fn test_partition_by() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build(
            "SELECT id, region, 
                    ROW_NUMBER() OVER (PARTITION BY region ORDER BY id)
             FROM sales",
        )
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 3);
}

// =============================================================================
// GROUP BY Tests
// =============================================================================

#[test]
fn test_group_by_with_aggregates() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build(
            "SELECT region, SUM(amount), COUNT(*)
             FROM sales
             GROUP BY region",
        )
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 3);
    assert!(!result[0].nullability); // region
    assert!(result[1].nullability); // SUM
    assert!(!result[2].nullability); // COUNT(*)
}

#[test]
fn test_having_clause() {
    let schema = setup_schema();
    let plan = BuildContext::new(&schema)
        .build(
            "SELECT region, SUM(amount) as total
             FROM sales
             GROUP BY region
             HAVING SUM(amount) > 100",
        )
        .unwrap();
    let result = plan.columns(&schema);

    assert_eq!(result.len(), 2);
}
