//! VALUES clause tests

use sqlex_common::DataType;
use sqlex_static_analyzer::{BuildContext, Dialect, Schema};

#[test]
fn test_values_clause_infer_types() {
    let schema = Schema::new(Dialect::PostgreSQL);

    // Test VALUES with single row
    let plan = BuildContext::new(&schema)
        .build("VALUES (1, 'hello')")
        .unwrap();
    let result = plan.columns();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].data_type, DataType::Int);
    assert_eq!(result[1].data_type, DataType::Text);
}

#[test]
fn test_values_clause_multiple_rows() {
    let schema = Schema::new(Dialect::PostgreSQL);

    // Test VALUES with multiple rows
    let plan = BuildContext::new(&schema)
        .build("VALUES (1, 10), (2, 20)")
        .unwrap();
    let result = plan.columns();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].data_type, DataType::Int);
    assert_eq!(result[1].data_type, DataType::Int);
}

#[test]
fn test_values_clause_nullability() {
    let schema = Schema::new(Dialect::PostgreSQL);

    // Test VALUES with NULLs
    let plan = BuildContext::new(&schema)
        .build("VALUES (1), (NULL)")
        .unwrap();
    let result = plan.columns();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].data_type, DataType::Int);
    assert!(result[0].nullability);
}
