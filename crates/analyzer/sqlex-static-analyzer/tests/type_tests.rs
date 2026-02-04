//! Type Inference Tests
//!
//! Tests for expression and function type inference

mod test_utils;
use sqlex_common::DataType;
use sqlex_static_analyzer::{Catalog, ColumnDef, Dialect, TableDef};
use test_utils::analyze_columns;

fn setup_schema() -> Catalog {
    let mut schema = Catalog::new(Dialect::PostgreSQL);

    let data = TableDef {
        name: "data".to_string(),
        columns: vec![
            ColumnDef::new("int_val", DataType::Int),
            ColumnDef::new("bigint_val", DataType::BigInt),
            ColumnDef::new("float_val", DataType::Float),
            ColumnDef::new("double_val", DataType::Double),
            ColumnDef::new("text_val", DataType::Text),
            ColumnDef::new("bool_val", DataType::Bool),
            ColumnDef::new("decimal_val", DataType::Decimal),
        ],
        primary_key: None,
        unique_constraints: vec![],
        foreign_keys: vec![],
    };
    schema.add_table(data);
    schema
}
// =============================================================================
// Arithmetic Type Promotion
// =============================================================================

#[test]
fn test_int_plus_bigint() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT int_val + bigint_val FROM data");

    assert_eq!(result[0].data_type, DataType::BigInt);
}

#[test]
fn test_int_plus_float() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT int_val + float_val FROM data");

    assert_eq!(result[0].data_type, DataType::Float);
}

#[test]
fn test_float_plus_double() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT float_val + double_val FROM data");

    assert_eq!(result[0].data_type, DataType::Double);
}

#[test]
fn test_int_plus_decimal() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT int_val + decimal_val FROM data");

    assert_eq!(result[0].data_type, DataType::Decimal);
}

// =============================================================================
// Division
// =============================================================================

#[test]
fn test_division_returns_double() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT int_val / 2 FROM data");

    assert_eq!(result[0].data_type, DataType::Double);
}

// =============================================================================
// Comparison Operations
// =============================================================================

#[test]
fn test_comparison_returns_bool() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT int_val > 0 FROM data");

    assert_eq!(result[0].data_type, DataType::Bool);
}

#[test]
fn test_equality_returns_bool() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT int_val = bigint_val FROM data");

    assert_eq!(result[0].data_type, DataType::Bool);
}

#[test]
fn test_logical_and_returns_bool() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT bool_val AND TRUE FROM data");

    assert_eq!(result[0].data_type, DataType::Bool);
}

// =============================================================================
// Aggregate Return Types
// =============================================================================

#[test]
fn test_count_returns_bigint() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT COUNT(*) FROM data");

    assert_eq!(result[0].data_type, DataType::BigInt);
}

#[test]
fn test_sum_int_returns_bigint() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT SUM(int_val) FROM data");

    assert_eq!(result[0].data_type, DataType::BigInt);
}

#[test]
fn test_sum_float_returns_double() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT SUM(float_val) FROM data");

    assert_eq!(result[0].data_type, DataType::Double);
}

#[test]
fn test_avg_returns_double() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT AVG(int_val) FROM data");

    assert_eq!(result[0].data_type, DataType::Double);
}

#[test]
fn test_min_preserves_type() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT MIN(int_val) FROM data");

    assert_eq!(result[0].data_type, DataType::Int);
}

#[test]
fn test_max_preserves_type() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT MAX(text_val) FROM data");

    assert_eq!(result[0].data_type, DataType::Text);
}

// =============================================================================
// String Operations
// =============================================================================

#[test]
fn test_string_concat() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT text_val || ' suffix' FROM data");

    assert_eq!(result[0].data_type, DataType::Text);
}

// =============================================================================
// Literal Types
// =============================================================================

#[test]
fn test_integer_literal_type() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT 42 FROM data");

    assert_eq!(result[0].data_type, DataType::Int);
}

#[test]
fn test_float_literal_type() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT 3.14 FROM data");

    assert_eq!(result[0].data_type, DataType::Double);
}

#[test]
fn test_string_literal_type() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT 'hello' FROM data");

    assert_eq!(result[0].data_type, DataType::Text);
}

#[test]
fn test_bool_literal_type() {
    let schema = setup_schema();
    let result = analyze_columns(&schema, "SELECT TRUE FROM data");

    assert_eq!(result[0].data_type, DataType::Bool);
}
