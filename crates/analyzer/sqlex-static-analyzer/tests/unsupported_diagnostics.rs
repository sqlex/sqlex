use sqlex_analyzer::{Analyzer, AnalyzerError};
use sqlex_common::dialect::Dialect;
use sqlex_static_analyzer::StaticAnalyzer;

fn assert_error_code(error: AnalyzerError, phase: &str, code: &str) {
    let rendered = error.to_string();
    let expected = format!("[{phase}:{code}]");
    assert!(
        rendered.contains(&expected),
        "expected error to contain '{expected}', got '{rendered}'"
    );
}

async fn execute_all(analyzer: &mut StaticAnalyzer, statements: &[&str]) {
    for statement in statements {
        analyzer
            .execute(statement)
            .await
            .unwrap_or_else(|error| panic!("setup failed for '{statement}': {error}"));
    }
}

#[tokio::test]
async fn analyze_rejects_multiple_from_items() {
    let mut analyzer = StaticAnalyzer::new(Dialect::Postgres);
    execute_all(
        &mut analyzer,
        &[
            "CREATE TABLE t1 (id INT PRIMARY KEY)",
            "CREATE TABLE t2 (id INT PRIMARY KEY)",
        ],
    )
    .await;

    let error = analyzer
        .analyze("SELECT * FROM t1, t2")
        .await
        .expect_err("query should fail");
    assert_error_code(error, "ALGEBRAIZE", "A3071");
}

#[tokio::test]
async fn analyze_rejects_lateral_derived_table() {
    let mut analyzer = StaticAnalyzer::new(Dialect::Postgres);
    execute_all(&mut analyzer, &["CREATE TABLE t1 (id INT PRIMARY KEY)"]).await;

    let error = analyzer
        .analyze("SELECT * FROM t1 JOIN LATERAL (SELECT t1.id) AS d(id) ON TRUE")
        .await
        .expect_err("query should fail");
    assert_error_code(error, "ALGEBRAIZE", "A3062");
}

#[tokio::test]
async fn analyze_requires_alias_for_derived_table() {
    let analyzer = StaticAnalyzer::new(Dialect::Postgres);

    let error = analyzer
        .analyze("SELECT * FROM (SELECT 1)")
        .await
        .expect_err("query should fail");
    assert_error_code(error, "ALGEBRAIZE", "A3063");
}

#[tokio::test]
async fn analyze_rejects_unsupported_table_factor_kind() {
    let mut analyzer = StaticAnalyzer::new(Dialect::Postgres);
    execute_all(
        &mut analyzer,
        &[
            "CREATE TABLE t1 (id INT PRIMARY KEY)",
            "CREATE TABLE t2 (id INT PRIMARY KEY)",
        ],
    )
    .await;

    let error = analyzer
        .analyze("SELECT * FROM (t1 JOIN t2 ON t1.id = t2.id)")
        .await
        .expect_err("query should fail");
    assert_error_code(error, "ALGEBRAIZE", "A3064");
}

#[tokio::test]
async fn analyze_rejects_unsupported_set_expression() {
    let analyzer = StaticAnalyzer::new(Dialect::Postgres);

    let error = analyzer
        .analyze("VALUES (1)")
        .await
        .expect_err("query should fail");
    assert_error_code(error, "ALGEBRAIZE", "A3065");
}

#[tokio::test]
async fn analyze_rejects_recursive_cte_without_select_seed() {
    let analyzer = StaticAnalyzer::new(Dialect::Postgres);

    let error = analyzer
        .analyze(
            "WITH RECURSIVE t(n) AS (VALUES (1) UNION ALL SELECT n + 1 FROM t WHERE n < 3) SELECT n FROM t",
        )
        .await
        .expect_err("query should fail");
    assert_error_code(error, "ALGEBRAIZE", "A3067");
}

#[tokio::test]
async fn analyze_rejects_unsupported_unary_operator() {
    let analyzer = StaticAnalyzer::new(Dialect::Postgres);

    let error = analyzer
        .analyze("SELECT ~1")
        .await
        .expect_err("query should fail");
    assert_error_code(error, "ALGEBRAIZE", "A3068");
}

#[tokio::test]
async fn analyze_rejects_unsupported_binary_operator() {
    let analyzer = StaticAnalyzer::new(Dialect::Postgres);

    let error = analyzer
        .analyze("SELECT 1 || 2")
        .await
        .expect_err("query should fail");
    assert_error_code(error, "ALGEBRAIZE", "A3069");
}

#[tokio::test]
async fn analyze_rejects_unsupported_scalar_expression() {
    let analyzer = StaticAnalyzer::new(Dialect::Postgres);

    let error = analyzer
        .analyze("SELECT INTERVAL '1 DAY'")
        .await
        .expect_err("query should fail");
    assert_error_code(error, "ALGEBRAIZE", "A3070");
}

#[tokio::test]
async fn analyze_rejects_unsupported_literal_kind() {
    let analyzer = StaticAnalyzer::new(Dialect::Postgres);

    let error = analyzer
        .analyze("SELECT X'AB'")
        .await
        .expect_err("query should fail");
    assert_error_code(error, "ALGEBRAIZE", "A3073");
}

#[tokio::test]
async fn execute_rejects_create_table_as_select() {
    let mut analyzer = StaticAnalyzer::new(Dialect::Postgres);

    let error = analyzer
        .execute("CREATE TABLE t AS SELECT 1")
        .await
        .expect_err("statement should fail");
    assert_error_code(error, "CATALOG", "C2002");
}

#[tokio::test]
async fn execute_rejects_unsupported_alter_table_operation() {
    let mut analyzer = StaticAnalyzer::new(Dialect::Postgres);
    execute_all(&mut analyzer, &["CREATE TABLE t (id INT PRIMARY KEY)"]).await;

    let error = analyzer
        .execute("ALTER TABLE t RENAME TO t2")
        .await
        .expect_err("statement should fail");
    assert_error_code(error, "CATALOG", "C2009");
}

#[tokio::test]
async fn execute_rejects_drop_non_table_object() {
    let mut analyzer = StaticAnalyzer::new(Dialect::Postgres);

    let error = analyzer
        .execute("DROP VIEW v1")
        .await
        .expect_err("statement should fail");
    assert_error_code(error, "CATALOG", "C2010");
}

#[tokio::test]
async fn sqlite_rejects_add_constraint_in_alter_table() {
    let mut analyzer = StaticAnalyzer::new(Dialect::SQLite);
    execute_all(&mut analyzer, &["CREATE TABLE t (id INTEGER PRIMARY KEY)"]).await;

    let error = analyzer
        .execute("ALTER TABLE t ADD CONSTRAINT uq_t UNIQUE (id)")
        .await
        .expect_err("statement should fail");
    assert_error_code(error, "CATALOG", "C2026");
}
