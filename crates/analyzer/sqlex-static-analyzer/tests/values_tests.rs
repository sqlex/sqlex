use sqlex_common::DataType;
use sqlex_static_analyzer::{
    analyzer::QueryAnalyzer,
    schema::{Dialect, Schema},
};

#[test]
fn test_values_clause_infer_types() {
    let schema = Schema::new(Dialect::PostgreSQL);
    let mut analyzer = QueryAnalyzer::new(&schema);

    // Test VALUES with single row
    let plan = analyzer.analyze("VALUES (1, 'hello')").unwrap();

    // Result set is actually PlanNode::Values wrapped in extraction logic?
    // Wait, analyze returns ResultSet, not PlanNode.
    // ResultSet has columns.

    assert_eq!(plan.columns.len(), 2);
    assert_eq!(plan.columns[0].data_type, DataType::Int);
    assert_eq!(plan.columns[1].data_type, DataType::Text);
}

#[test]
fn test_values_clause_multiple_rows() {
    let schema = Schema::new(Dialect::PostgreSQL);
    let mut analyzer = QueryAnalyzer::new(&schema);

    // Test VALUES with multiple rows
    let plan = analyzer.analyze("VALUES (1, 10), (2, 20)").unwrap();

    assert_eq!(plan.columns.len(), 2);
    assert_eq!(plan.columns[0].data_type, DataType::Int);
    assert_eq!(plan.columns[1].data_type, DataType::Int);
}

#[test]
fn test_values_clause_nullability() {
    let schema = Schema::new(Dialect::PostgreSQL);
    let mut analyzer = QueryAnalyzer::new(&schema);

    // Test VALUES with NULLs
    let plan = analyzer.analyze("VALUES (1), (NULL)").unwrap();

    assert_eq!(plan.columns.len(), 1);
    assert_eq!(plan.columns[0].data_type, DataType::Int);
    assert!(plan.columns[0].nullability);
}
