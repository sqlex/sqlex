//! Algebraizer error code definitions.
//!
//! Numbering policy:
//! - Format: `<module><major><minor>`, where module is `A` for algebraizer.
//! - `<major>`: 2 digits (00-99), top-level category.
//! - `<minor>`: 2 digits (00-99), sub-category.
//! - Algebraizer majors:
//!   - `00`: scalar/column resolution baseline
//!   - `01`: projection/alias/grouping binding
//!   - `02`: function and CTE semantic checks
//!   - `03`: ORDER BY / set-op / subquery checks
//!   - `04`: aggregate/window semantics
//!   - `05`: join and dialect-specific join/function checks
//!   - `06`: unsupported SQL features in current algebraizer path
//!   - `07`: terminal unsupported/algebraizer path checks

/// Only query statements are supported by algebraizer entry.
pub(crate) const DISPATCH_ONLY_QUERY_STATEMENT_SUPPORTED: &str = "A0001";
/// A qualified wildcard references an unknown table alias.
pub(crate) const SELECT_UNKNOWN_QUALIFIED_WILDCARD_TARGET: &str = "A0002";
/// A referenced table cannot be found in catalog.
pub(crate) const FROM_TABLE_NOT_FOUND: &str = "A0003";
/// A compound identifier has no segments.
pub(crate) const EXPRESSION_EMPTY_COMPOUND_IDENTIFIER: &str = "A0004";
/// A floating-point literal cannot be parsed.
pub(crate) const LITERAL_INVALID_FLOAT: &str = "A0005";
/// An integer literal cannot be parsed.
pub(crate) const LITERAL_INVALID_INTEGER: &str = "A0006";
/// A column reference cannot be resolved in current scope.
pub(crate) const COLUMN_NOT_FOUND: &str = "A0008";
/// A column reference matches more than one candidate.
pub(crate) const COLUMN_REFERENCE_AMBIGUOUS: &str = "A0009";

/// A relation reference is invalid in current context.
pub(crate) const RELATION_REFERENCE_INVALID: &str = "A0100";
/// A qualified column cannot be resolved to a relation.
pub(crate) const QUALIFIED_COLUMN_NOT_FOUND: &str = "A0101";
/// A projection item contains an empty compound identifier.
pub(crate) const PROJECTION_EMPTY_COMPOUND_IDENTIFIER: &str = "A0102";
/// Derived-table alias column count does not match output columns.
pub(crate) const DERIVED_TABLE_ALIAS_COLUMN_COUNT_MISMATCH: &str = "A0103";
/// CTE alias column count does not match CTE output columns.
pub(crate) const CTE_COLUMN_ALIAS_COUNT_MISMATCH: &str = "A0104";
/// Recursive CTE alias column count does not match recursive output.
pub(crate) const RECURSIVE_CTE_COLUMN_ALIAS_COUNT_MISMATCH: &str = "A0105";
/// Aggregate expressions are not allowed in GROUP BY.
pub(crate) const GROUP_BY_AGGREGATE_NOT_ALLOWED: &str = "A0106";
/// Non-aggregated projection appears without valid grouping.
pub(crate) const PROJECTION_NON_AGGREGATED_WITHOUT_GROUP_BY: &str = "A0107";
/// LIMIT/OFFSET clause has an invalid numeric literal.
pub(crate) const CLAUSE_INVALID_NUMERIC_VALUE: &str = "A0108";
/// Set-operation branches do not produce the same column count.
pub(crate) const SET_OPERATION_COLUMN_COUNT_MISMATCH: &str = "A0109";

/// Function call has fewer arguments than required.
pub(crate) const FUNCTION_ARGUMENTS_TOO_FEW: &str = "A0200";
/// Function call has more arguments than allowed.
pub(crate) const FUNCTION_ARGUMENTS_TOO_MANY: &str = "A0201";
/// Alias uses a reserved keyword in current dialect/context.
pub(crate) const ALIAS_RESERVED_KEYWORD: &str = "A0204";
/// Duplicate CTE name appears in one WITH block.
pub(crate) const CTE_DUPLICATE_NAME: &str = "A0205";
/// Recursive CTE seed/recursive terms output different column counts.
pub(crate) const RECURSIVE_CTE_TERM_COLUMN_COUNT_MISMATCH: &str = "A0206";
/// JOIN USING requires at least one shared column.
pub(crate) const JOIN_USING_REQUIRES_SHARED_COLUMN: &str = "A0207";

/// ORDER BY ... INTERPOLATE is not supported.
pub(crate) const ORDER_BY_INTERPOLATE_UNSUPPORTED: &str = "A0300";
/// ORDER BY ... WITH FILL is not supported.
pub(crate) const ORDER_BY_WITH_FILL_UNSUPPORTED: &str = "A0301";
/// ORDER BY position must start from 1.
pub(crate) const ORDER_BY_POSITION_INVALID: &str = "A0302";
/// ORDER BY position exceeds projection column count.
pub(crate) const ORDER_BY_POSITION_OUT_OF_RANGE: &str = "A0303";
/// Scalar subquery must produce exactly one column.
pub(crate) const SUBQUERY_EXPECTS_SINGLE_COLUMN: &str = "A0305";

/// Aggregate expressions are not allowed in WHERE.
pub(crate) const WHERE_AGGREGATE_NOT_ALLOWED: &str = "A0401";
/// Window expressions are not allowed in WHERE.
pub(crate) const WHERE_WINDOW_NOT_ALLOWED: &str = "A0402";
/// Window expressions are not allowed in HAVING.
pub(crate) const HAVING_WINDOW_NOT_ALLOWED: &str = "A0403";
/// Projection expression is neither grouped nor aggregated.
pub(crate) const PROJECTION_NOT_GROUPED_OR_AGGREGATED: &str = "A0404";
/// HAVING expression is neither grouped nor aggregated.
pub(crate) const HAVING_NOT_GROUPED_OR_AGGREGATED: &str = "A0405";
/// Named window is defined more than once.
pub(crate) const WINDOW_DEFINITION_DUPLICATE: &str = "A0406";
/// Named windows contain a cyclic reference.
pub(crate) const WINDOW_DEFINITION_CYCLIC: &str = "A0407";
/// Named window reference cannot be resolved.
pub(crate) const WINDOW_DEFINITION_NOT_FOUND: &str = "A0408";
/// Under DISTINCT, ORDER BY expression must appear in SELECT list.
pub(crate) const ORDER_BY_EXPRESSION_NOT_IN_SELECT_UNDER_DISTINCT: &str = "A0409";

/// LIMIT/OFFSET requires a non-negative integer literal.
pub(crate) const CLAUSE_EXPECTS_NON_NEGATIVE_INTEGER_LITERAL: &str = "A0500";
/// Advanced SELECT clauses are not supported in this analyzer path.
pub(crate) const SELECT_ADVANCED_CLAUSES_UNSUPPORTED: &str = "A0501";
/// GROUP BY expression form is unsupported.
pub(crate) const GROUP_BY_FORM_UNSUPPORTED: &str = "A0502";
/// GLOBAL JOIN is unsupported.
pub(crate) const GLOBAL_JOIN_UNSUPPORTED: &str = "A0503";
/// MySQL FULL JOIN is unsupported.
pub(crate) const MYSQL_FULL_JOIN_UNSUPPORTED: &str = "A0504";
/// JOIN operator is unsupported.
pub(crate) const JOIN_OPERATOR_UNSUPPORTED: &str = "A0505";
/// NATURAL JOIN is unsupported.
pub(crate) const NATURAL_JOIN_UNSUPPORTED: &str = "A0506";
/// Internal algebraizer invariant is violated.
pub(crate) const INTERNAL_INVARIANT_VIOLATED: &str = "A0507";
/// Function OVER clause shape is unsupported.
pub(crate) const FUNCTION_OVER_CLAUSE_UNSUPPORTED: &str = "A0508";
/// Window function requires an OVER clause.
pub(crate) const WINDOW_FUNCTION_OVER_REQUIRED: &str = "A0509";

/// CEIL/FLOOR modifiers are unsupported.
pub(crate) const CEIL_FLOOR_MODIFIERS_UNSUPPORTED: &str = "A0600";
/// TRIM modifiers are unsupported.
pub(crate) const TRIM_MODIFIERS_UNSUPPORTED: &str = "A0601";
/// LATERAL derived table is unsupported.
pub(crate) const LATERAL_DERIVED_TABLE_UNSUPPORTED: &str = "A0602";
/// Derived table requires an explicit alias.
pub(crate) const DERIVED_TABLE_ALIAS_REQUIRED: &str = "A0603";
/// FROM table factor shape is unsupported.
pub(crate) const TABLE_FACTOR_UNSUPPORTED: &str = "A0604";
/// Set expression shape is unsupported.
pub(crate) const SET_EXPRESSION_UNSUPPORTED: &str = "A0605";
/// CTE SEARCH/CYCLE clause is unsupported.
pub(crate) const CTE_SEARCH_CYCLE_UNSUPPORTED: &str = "A0606";
/// Recursive CTE seed query is not SELECT-compatible.
pub(crate) const RECURSIVE_CTE_SEED_NOT_SELECT_COMPATIBLE: &str = "A0607";
/// Unary operator is unsupported.
pub(crate) const UNARY_OPERATOR_UNSUPPORTED: &str = "A0608";
/// Binary operator is unsupported.
pub(crate) const BINARY_OPERATOR_UNSUPPORTED: &str = "A0609";

/// Scalar expression variant is unsupported.
pub(crate) const SCALAR_EXPRESSION_UNSUPPORTED: &str = "A0700";
/// Multiple FROM items are unsupported in this path.
pub(crate) const MULTIPLE_FROM_ITEMS_UNSUPPORTED: &str = "A0701";
/// Literal variant is unsupported.
pub(crate) const LITERAL_UNSUPPORTED: &str = "A0703";
