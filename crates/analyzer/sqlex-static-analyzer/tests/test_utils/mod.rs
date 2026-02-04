#![allow(dead_code)]

use sqlex_static_analyzer::{
    Catalog,
    analysis::{self, diagnostics::DiagnosticSeverity},
};

pub fn analyze_ok(catalog: &Catalog, sql: &str) -> sqlex_static_analyzer::ir::OutputSchema {
    let result = analysis::analyze(catalog, sql);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Expected analysis to succeed, got errors: {}",
        errors
            .iter()
            .map(|d| d.message.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    );
    result.output.expect("Expected output schema")
}

pub fn analyze_columns(
    catalog: &Catalog,
    sql: &str,
) -> Vec<sqlex_static_analyzer::ir::OutputColumn> {
    analyze_ok(catalog, sql).columns
}

pub fn analyze_err(catalog: &Catalog, sql: &str) {
    let result = analysis::analyze(catalog, sql);
    let has_error = result
        .diagnostics
        .iter()
        .any(|d| d.severity == DiagnosticSeverity::Error);
    assert!(has_error, "Expected analysis error, got none");
}
