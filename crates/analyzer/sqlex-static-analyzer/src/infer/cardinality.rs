use std::collections::{HashMap, HashSet};

use sqlex_common::types::Cardinality;

use super::Inferrer;
use crate::ir::{
    auxiliary::{BinaryOp, JoinKind},
    relational::RelationalExpr,
    scalar::{LiteralValue, ScalarExpr},
};

// ------------------------------------------------------------------
// Cardinality combinators
// ------------------------------------------------------------------

pub(super) fn constrain_at_most_one(cardinality: Cardinality) -> Cardinality {
    match cardinality {
        Cardinality::ExactlyOne => Cardinality::ExactlyOne,
        Cardinality::AtLeastOne => Cardinality::ExactlyOne,
        Cardinality::AtMostOne => Cardinality::AtMostOne,
        Cardinality::Unknown => Cardinality::AtMostOne,
    }
}

pub(super) fn drop_lower_bound(cardinality: Cardinality) -> Cardinality {
    match cardinality {
        Cardinality::ExactlyOne => Cardinality::AtMostOne,
        Cardinality::AtLeastOne => Cardinality::Unknown,
        Cardinality::AtMostOne => Cardinality::AtMostOne,
        Cardinality::Unknown => Cardinality::Unknown,
    }
}

pub(super) fn max_cardinality(left: Cardinality, right: Cardinality) -> Cardinality {
    match (left, right) {
        (Cardinality::AtLeastOne, _) | (_, Cardinality::AtLeastOne) => Cardinality::AtLeastOne,
        (Cardinality::ExactlyOne, _) | (_, Cardinality::ExactlyOne) => Cardinality::AtLeastOne,
        _ => Cardinality::Unknown,
    }
}

pub(super) fn min_cardinality(left: Cardinality, right: Cardinality) -> Cardinality {
    match (left, right) {
        (Cardinality::AtMostOne, _) | (_, Cardinality::AtMostOne) => Cardinality::AtMostOne,
        (Cardinality::ExactlyOne, Cardinality::ExactlyOne) => Cardinality::ExactlyOne,
        _ => Cardinality::Unknown,
    }
}

pub(super) fn combine_join_cardinality(
    left: Cardinality,
    right: Cardinality,
    join_kind: JoinKind,
) -> Cardinality {
    match join_kind {
        JoinKind::Inner | JoinKind::Cross => drop_lower_bound(left),
        JoinKind::Left => left,
        JoinKind::Right => right,
        JoinKind::Full => max_cardinality(left, right),
    }
}

// ------------------------------------------------------------------
// WHERE condition constraint analysis
// ------------------------------------------------------------------

/// Constraint information extracted from WHERE conditions.
#[derive(Debug, Default)]
struct ConstraintInfo {
    /// Equality constraints: (table, column) -> whether compared to a constant
    equality_constraints: Vec<(Option<String>, String)>,
    /// IN constraints: (table, column) -> list length
    in_constraints: Vec<(Option<String>, String, usize)>,
    /// IS NULL constraints
    is_null_constraints: Vec<(Option<String>, String)>,
}

impl Inferrer<'_> {
    /// Analyze a WHERE condition to determine if it constrains cardinality.
    /// Called on the full RelationalExpr tree to find Selection nodes and
    /// check their conditions against catalog constraints.
    pub(super) fn analyze_selection_cardinality(
        &self,
        input: &RelationalExpr,
        condition: &ScalarExpr,
    ) -> Option<Cardinality> {
        // Check for constant false
        if is_constant_false(condition) {
            return Some(Cardinality::AtMostOne);
        }

        // Extract constraints from the condition
        let mut constraints = ConstraintInfo::default();
        if !extract_constraints(condition, &mut constraints) {
            return None;
        }

        // Collect table names from the input expression
        let tables = collect_scan_tables(input);

        // Check equality constraints against primary keys
        if !constraints.equality_constraints.is_empty() {
            if let Some(card) = self.check_pk_match(&constraints, &tables) {
                return Some(card);
            }
            if let Some(card) = self.check_unique_match(&constraints, &tables) {
                return Some(card);
            }
        }

        // Check IN constraints
        if !constraints.in_constraints.is_empty() {
            if let Some(card) = self.check_in_constraint(&constraints, &tables) {
                return Some(card);
            }
        }

        // Check IS NULL constraints
        if !constraints.is_null_constraints.is_empty() {
            if let Some(card) = self.check_is_null_constraint(&constraints, &tables) {
                return Some(card);
            }
        }

        None
    }

    fn check_pk_match(
        &self,
        constraints: &ConstraintInfo,
        tables: &HashMap<Option<String>, String>,
    ) -> Option<Cardinality> {
        // Group equality constraints by resolved table name
        let mut table_columns: HashMap<&str, HashSet<&str>> = HashMap::new();

        for (tbl_ref, col_name) in &constraints.equality_constraints {
            if let Some(real_table) = resolve_table(tbl_ref, tables) {
                table_columns
                    .entry(real_table)
                    .or_default()
                    .insert(col_name.as_str());
            }
        }

        for (table_name, constrained_cols) in &table_columns {
            if let Some(table_def) = self.catalog.get_table(table_name) {
                if let Some(pk_columns) = &table_def.primary_key {
                    if pk_columns
                        .iter()
                        .all(|pk| constrained_cols.contains(pk.as_str()))
                    {
                        return Some(Cardinality::AtMostOne);
                    }
                }
            }
        }

        None
    }

    fn check_unique_match(
        &self,
        constraints: &ConstraintInfo,
        tables: &HashMap<Option<String>, String>,
    ) -> Option<Cardinality> {
        let mut table_columns: HashMap<&str, HashSet<&str>> = HashMap::new();

        for (tbl_ref, col_name) in &constraints.equality_constraints {
            if let Some(real_table) = resolve_table(tbl_ref, tables) {
                table_columns
                    .entry(real_table)
                    .or_default()
                    .insert(col_name.as_str());
            }
        }

        for (table_name, constrained_cols) in &table_columns {
            if let Some(table_def) = self.catalog.get_table(table_name) {
                for unique_constraint in &table_def.unique_constraints {
                    if unique_constraint
                        .iter()
                        .all(|uc| constrained_cols.contains(uc.as_str()))
                    {
                        return Some(Cardinality::AtMostOne);
                    }
                }
            }
        }

        None
    }

    fn check_in_constraint(
        &self,
        constraints: &ConstraintInfo,
        tables: &HashMap<Option<String>, String>,
    ) -> Option<Cardinality> {
        for (tbl_ref, col_name, list_len) in &constraints.in_constraints {
            let Some(real_table) = resolve_table(tbl_ref, tables) else {
                continue;
            };
            let Some(table_def) = self.catalog.get_table(real_table) else {
                continue;
            };

            let is_pk = table_def
                .primary_key
                .as_ref()
                .map(|pk| pk.len() == 1 && pk[0].eq_ignore_ascii_case(col_name))
                .unwrap_or(false);

            let is_unique = table_def
                .unique_constraints
                .iter()
                .any(|uc| uc.len() == 1 && uc[0].eq_ignore_ascii_case(col_name));

            if is_pk || is_unique {
                if *list_len <= 1 {
                    return Some(Cardinality::AtMostOne);
                }
                return Some(Cardinality::Unknown);
            }
        }
        None
    }

    fn check_is_null_constraint(
        &self,
        constraints: &ConstraintInfo,
        tables: &HashMap<Option<String>, String>,
    ) -> Option<Cardinality> {
        for (tbl_ref, col_name) in &constraints.is_null_constraints {
            let Some(real_table) = resolve_table(tbl_ref, tables) else {
                continue;
            };
            let Some(table_def) = self.catalog.get_table(real_table) else {
                continue;
            };

            if let Some(pk_columns) = &table_def.primary_key {
                if pk_columns
                    .iter()
                    .any(|pk| pk.eq_ignore_ascii_case(col_name))
                {
                    return Some(Cardinality::AtMostOne);
                }
            }
        }
        None
    }
}

// ------------------------------------------------------------------
// Helpers
// ------------------------------------------------------------------

fn is_constant_false(expr: &ScalarExpr) -> bool {
    match expr {
        ScalarExpr::Literal(LiteralValue::Boolean(false)) => true,
        ScalarExpr::BinaryOp {
            left,
            op: BinaryOp::Eq,
            right,
        } => {
            if let (ScalarExpr::Literal(l), ScalarExpr::Literal(r)) =
                (left.as_ref(), right.as_ref())
            {
                matches!(
                    (l, r),
                    (LiteralValue::Integer(a), LiteralValue::Integer(b)) if a != b
                ) || matches!(
                    (l, r),
                    (LiteralValue::String(a), LiteralValue::String(b)) if a != b
                )
            } else {
                false
            }
        },
        _ => false,
    }
}

/// Extract constraints from a WHERE expression.
/// Returns false if the expression contains OR or other patterns that
/// make constraints unreliable for cardinality analysis.
fn extract_constraints(expr: &ScalarExpr, constraints: &mut ConstraintInfo) -> bool {
    match expr {
        ScalarExpr::BinaryOp {
            left,
            op: BinaryOp::Eq,
            right,
        } => match (left.as_ref(), right.as_ref()) {
            (ScalarExpr::ColumnRef { table, column }, ScalarExpr::Literal(_)) => {
                constraints
                    .equality_constraints
                    .push((table.clone(), column.clone()));
                true
            },
            (ScalarExpr::Literal(_), ScalarExpr::ColumnRef { table, column }) => {
                constraints
                    .equality_constraints
                    .push((table.clone(), column.clone()));
                true
            },
            _ => false,
        },
        ScalarExpr::BinaryOp {
            left,
            op: BinaryOp::And,
            right,
        } => {
            let left_ok = extract_constraints(left, constraints);
            let right_ok = extract_constraints(right, constraints);
            left_ok && right_ok
        },
        ScalarExpr::BinaryOp {
            op: BinaryOp::Or, ..
        } => false,
        ScalarExpr::InList {
            expr,
            list,
            negated: false,
        } => {
            if let ScalarExpr::ColumnRef { table, column } = expr.as_ref() {
                let all_literals = list
                    .iter()
                    .all(|item| matches!(item, ScalarExpr::Literal(_)));
                if all_literals {
                    constraints
                        .in_constraints
                        .push((table.clone(), column.clone(), list.len()));
                    return true;
                }
            }
            false
        },
        ScalarExpr::IsNull {
            expr,
            negated: false,
        } => {
            if let ScalarExpr::ColumnRef { table, column } = expr.as_ref() {
                constraints
                    .is_null_constraints
                    .push((table.clone(), column.clone()));
                true
            } else {
                false
            }
        },
        _ => false,
    }
}

/// Collect all Scan table names from a RelationalExpr tree.
/// Returns a map from alias (or None) to the real table name.
fn collect_scan_tables(expr: &RelationalExpr) -> HashMap<Option<String>, String> {
    let mut tables = HashMap::new();
    collect_scan_tables_inner(expr, &mut tables);
    tables
}

fn collect_scan_tables_inner(expr: &RelationalExpr, tables: &mut HashMap<Option<String>, String>) {
    match expr {
        RelationalExpr::Scan { table, alias } => {
            // Map alias -> real table name, and None -> real table name
            if let Some(alias) = alias {
                tables.insert(Some(alias.clone()), table.clone());
            }
            tables.insert(Some(table.clone()), table.clone());
            // Also insert None mapping if there's only one table
            // (will be overwritten if multiple tables exist, but that's fine
            // since None-qualified columns are ambiguous with multiple tables)
            tables.insert(None, table.clone());
        },
        RelationalExpr::Alias { input, .. }
        | RelationalExpr::Selection { input, .. }
        | RelationalExpr::Projection { input, .. }
        | RelationalExpr::Aggregation { input, .. }
        | RelationalExpr::Window { input, .. }
        | RelationalExpr::Distinct { input }
        | RelationalExpr::Sort { input, .. }
        | RelationalExpr::Limit { input, .. } => {
            collect_scan_tables_inner(input, tables);
        },
        RelationalExpr::Join { left, right, .. }
        | RelationalExpr::SetOperation { left, right, .. } => {
            collect_scan_tables_inner(left, tables);
            collect_scan_tables_inner(right, tables);
        },
        RelationalExpr::Values { .. } => {},
    }
}

/// Resolve a column's table reference to a real table name.
fn resolve_table<'a>(
    tbl_ref: &Option<String>,
    tables: &'a HashMap<Option<String>, String>,
) -> Option<&'a str> {
    tables.get(tbl_ref).map(|s| s.as_str())
}
