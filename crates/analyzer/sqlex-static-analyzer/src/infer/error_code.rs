//! Inferencer error code definitions.
//!
//! Numbering policy:
//! - Format: `<module><major><minor>`, where module is `I` for inferencer.
//! - `<major>`: 2 digits (00-99), top-level category.
//! - `<minor>`: 2 digits (00-99), sub-category.
//! - Infer majors:
//!   - `01`: expression-level inference and validation
//!   - `02`: relation/cardinality-level inference

/// Slot reference cannot be resolved in infer metadata.
pub(crate) const SLOT_REFERENCE_UNKNOWN: &str = "I0101";
/// Aggregate function is unsupported in infer expression path.
pub(crate) const AGGREGATE_FUNCTION_UNSUPPORTED: &str = "I0102";
/// Window function is unsupported in infer expression path.
pub(crate) const WINDOW_FUNCTION_UNSUPPORTED: &str = "I0103";
/// Operator argument types are incompatible.
pub(crate) const OPERATOR_TYPE_MISMATCH: &str = "I0104";
/// Subquery expression must produce exactly one column.
pub(crate) const SUBQUERY_EXPECTS_SINGLE_COLUMN: &str = "I0105";
/// Correlated reference depth is invalid.
pub(crate) const CORRELATED_REFERENCE_DEPTH_INVALID: &str = "I0106";
/// Correlated slot reference cannot be resolved.
pub(crate) const CORRELATED_SLOT_REFERENCE_UNKNOWN: &str = "I0107";
/// Function requires a text argument.
pub(crate) const FUNCTION_EXPECTS_TEXT_ARGUMENT: &str = "I0108";
/// Function requires a numeric argument.
pub(crate) const FUNCTION_EXPECTS_NUMERIC_ARGUMENT: &str = "I0109";

/// Projection metadata is missing alias assignment for an output column.
pub(crate) const PROJECTION_ALIAS_NOT_ASSIGNED: &str = "I0201";
/// Set-operation branches do not produce the same column count.
pub(crate) const SET_OPERATION_COLUMN_COUNT_MISMATCH: &str = "I0202";
/// Cardinality interval is internally inconsistent.
pub(crate) const CARDINALITY_INTERVAL_INVALID: &str = "I0203";
