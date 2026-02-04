//! VALUES clause tests

mod test_utils;
use sqlex_common::DataType;
use sqlex_static_analyzer::{Catalog, Dialect};
use test_utils::analyze_columns;

#[test]
fn test_values_clause_infer_types() {
    let schema = Catalog::new(Dialect::PostgreSQL);

    // Test VALUES with single row
    let result = analyze_columns(&schema, "VALUES (1, 'hello')");

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].data_type, DataType::Int);
    assert_eq!(result[1].data_type, DataType::Text);
}

#[test]
fn test_values_clause_multiple_rows() {
    let schema = Catalog::new(Dialect::PostgreSQL);

    // Test VALUES with multiple rows
    let result = analyze_columns(&schema, "VALUES (1, 10), (2, 20)");

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].data_type, DataType::Int);
    assert_eq!(result[1].data_type, DataType::Int);
}

#[test]
fn test_values_clause_nullability() {
    let schema = Catalog::new(Dialect::PostgreSQL);

    // Test VALUES with NULLs
    let result = analyze_columns(&schema, "VALUES (1), (NULL)");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].data_type, DataType::Int);
    assert!(result[0].nullability);
}
