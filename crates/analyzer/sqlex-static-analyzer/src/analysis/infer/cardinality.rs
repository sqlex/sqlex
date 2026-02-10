use std::collections::{HashMap, HashSet};

use sqlex_common::types::Cardinality;
use sqlparser::ast::Value;

use crate::{
    analysis::{functions::Function, infer::Inferrer},
    ir::{
        bound::{BoundExpr, BoundQueryBody, BoundSelect, BoundSetExpr, BoundSetOp, BoundStatement},
        ids::{ColumnId, ExprId},
    },
};

/// Represents constraint information extracted from WHERE conditions
#[derive(Debug, Default)]
struct ConstraintInfo {
    /// Equality constraints: column -> whether it's compared to a constant
    equality_constraints: HashMap<ColumnId, bool>,
    /// IN constraints: column -> list length
    in_constraints: HashMap<ColumnId, usize>,
    /// IS NULL constraints: set of columns
    is_null_constraints: HashSet<ColumnId>,
}

fn constrain_at_most_one(cardinality: Cardinality) -> Cardinality {
    match cardinality {
        Cardinality::ExactlyOne => Cardinality::ExactlyOne,
        Cardinality::AtLeastOne => Cardinality::ExactlyOne,
        Cardinality::AtMostOne => Cardinality::AtMostOne,
        Cardinality::Unknown => Cardinality::AtMostOne,
    }
}

fn drop_lower_bound(cardinality: Cardinality) -> Cardinality {
    match cardinality {
        Cardinality::ExactlyOne => Cardinality::AtMostOne,
        Cardinality::AtLeastOne => Cardinality::Unknown,
        Cardinality::AtMostOne => Cardinality::AtMostOne,
        Cardinality::Unknown => Cardinality::Unknown,
    }
}

fn max_cardinality(left: Cardinality, right: Cardinality) -> Cardinality {
    match (left, right) {
        (Cardinality::AtLeastOne, _) | (_, Cardinality::AtLeastOne) => Cardinality::AtLeastOne,
        (Cardinality::ExactlyOne, _) | (_, Cardinality::ExactlyOne) => Cardinality::AtLeastOne,
        _ => Cardinality::Unknown,
    }
}

fn min_cardinality(left: Cardinality, right: Cardinality) -> Cardinality {
    match (left, right) {
        (Cardinality::AtMostOne, _) | (_, Cardinality::AtMostOne) => Cardinality::AtMostOne,
        (Cardinality::ExactlyOne, Cardinality::ExactlyOne) => Cardinality::ExactlyOne,
        _ => Cardinality::Unknown,
    }
}

fn combine_join_cardinality(
    left: Cardinality,
    right: Cardinality,
    join_kind: crate::ir::bound::BoundJoinKind,
) -> Cardinality {
    use crate::ir::bound::BoundJoinKind;

    match join_kind {
        BoundJoinKind::Inner | BoundJoinKind::Cross => drop_lower_bound(left),
        BoundJoinKind::Left => left,
        BoundJoinKind::Right => right,
        BoundJoinKind::Full => max_cardinality(left, right),
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct QueryFacts {
    has_from: bool,
    has_aggregate_without_group_by: bool,
    has_group_by: bool,
    has_distinct: bool,
    values_len: Option<usize>,
    limit: Option<u64>,
    offset: Option<u64>,
}

impl Inferrer<'_> {
    /// Get the TableDef for a column from the catalog
    fn get_table_def_for_column(
        &self,
        stmt: &BoundStatement,
        column_id: ColumnId,
    ) -> Option<&crate::catalog::types::TableDef> {
        let column = stmt.columns.get(column_id);
        let table = stmt.tables.get(column.table);

        match &table.source {
            crate::ir::bound::BoundTableSource::Table { name } => self.catalog.get_table(name),
            // CTE and derived tables don't have catalog information
            crate::ir::bound::BoundTableSource::Cte { .. }
            | crate::ir::bound::BoundTableSource::Derived { .. } => None,
        }
    }

    /// Extract constraint information from WHERE expression
    /// Returns false if the expression contains OR or other conditions that make constraints unreliable
    fn extract_constraints_from_expr(
        &self,
        stmt: &BoundStatement,
        expr_id: ExprId,
        constraints: &mut ConstraintInfo,
    ) -> bool {
        let expr = stmt.exprs.get(expr_id);

        match expr {
            // Handle equality: column = literal or literal = column
            BoundExpr::Binary {
                left,
                op: sqlparser::ast::BinaryOperator::Eq,
                right,
            } => {
                let left_expr = stmt.exprs.get(*left);
                let right_expr = stmt.exprs.get(*right);

                match (left_expr, right_expr) {
                    (BoundExpr::Column(col_id), BoundExpr::Literal(_)) => {
                        constraints.equality_constraints.insert(*col_id, true);
                        true
                    },
                    (BoundExpr::Literal(_), BoundExpr::Column(col_id)) => {
                        constraints.equality_constraints.insert(*col_id, true);
                        true
                    },
                    _ => false,
                }
            },
            // Handle AND: merge constraints from both sides
            BoundExpr::Binary {
                left,
                op: sqlparser::ast::BinaryOperator::And,
                right,
            } => {
                let left_ok = self.extract_constraints_from_expr(stmt, *left, constraints);
                let right_ok = self.extract_constraints_from_expr(stmt, *right, constraints);
                left_ok && right_ok
            },
            // Handle OR: constraints are unreliable
            BoundExpr::Binary {
                op: sqlparser::ast::BinaryOperator::Or,
                ..
            } => false,
            // Handle IN list
            BoundExpr::InList {
                expr,
                list,
                negated: false,
            } => {
                if let BoundExpr::Column(col_id) = stmt.exprs.get(*expr) {
                    // Check if all items in the list are literals
                    let all_literals = list
                        .iter()
                        .all(|item_id| matches!(stmt.exprs.get(*item_id), BoundExpr::Literal(_)));
                    if all_literals {
                        constraints.in_constraints.insert(*col_id, list.len());
                        return true;
                    }
                }
                false
            },
            // Handle IS NULL
            BoundExpr::IsNull {
                expr,
                negated: false,
            } => {
                if let BoundExpr::Column(col_id) = stmt.exprs.get(*expr) {
                    constraints.is_null_constraints.insert(*col_id);
                    true
                } else {
                    false
                }
            },
            _ => false,
        }
    }

    /// Check if equality constraints match a primary key
    fn check_primary_key_match(
        &self,
        stmt: &BoundStatement,
        constraints: &ConstraintInfo,
    ) -> Option<Cardinality> {
        // Group constraints by table
        let mut table_constraints: HashMap<crate::ir::ids::TableId, HashSet<String>> =
            HashMap::new();

        for col_id in constraints.equality_constraints.keys() {
            let column = stmt.columns.get(*col_id);
            table_constraints
                .entry(column.table)
                .or_default()
                .insert(column.name.clone());
        }

        // Check each table's primary key
        for (table_id, constrained_columns) in table_constraints {
            let table = stmt.tables.get(table_id);
            if let crate::ir::bound::BoundTableSource::Table { name } = &table.source {
                if let Some(table_def) = self.catalog.get_table(name) {
                    if let Some(pk_columns) = &table_def.primary_key {
                        // Check if all primary key columns are in equality constraints
                        // Additional constraints are fine (e.g., WHERE id = 1 AND name = 'John')
                        if pk_columns
                            .iter()
                            .all(|pk_col| constrained_columns.contains(pk_col))
                        {
                            return Some(Cardinality::AtMostOne);
                        }
                    }
                }
            }
        }

        None
    }

    /// Check if equality constraints match a unique constraint
    fn check_unique_constraint_match(
        &self,
        stmt: &BoundStatement,
        constraints: &ConstraintInfo,
    ) -> Option<Cardinality> {
        // Group constraints by table
        let mut table_constraints: HashMap<crate::ir::ids::TableId, HashSet<String>> =
            HashMap::new();

        for col_id in constraints.equality_constraints.keys() {
            let column = stmt.columns.get(*col_id);
            table_constraints
                .entry(column.table)
                .or_default()
                .insert(column.name.clone());
        }

        // Check each table's unique constraints
        for (table_id, constrained_columns) in table_constraints {
            let table = stmt.tables.get(table_id);
            if let crate::ir::bound::BoundTableSource::Table { name } = &table.source {
                if let Some(table_def) = self.catalog.get_table(name) {
                    for unique_constraint in &table_def.unique_constraints {
                        // Check if all unique constraint columns are in equality constraints
                        // Additional constraints are fine
                        if unique_constraint
                            .iter()
                            .all(|uc_col| constrained_columns.contains(uc_col))
                        {
                            return Some(Cardinality::AtMostOne);
                        }
                    }
                }
            }
        }

        None
    }

    /// Analyze IN constraint for cardinality
    fn analyze_in_constraint(
        &self,
        stmt: &BoundStatement,
        constraints: &ConstraintInfo,
    ) -> Option<Cardinality> {
        for (col_id, list_len) in &constraints.in_constraints {
            // Check if this column is a primary key or unique constraint (single column)
            if let Some(table_def) = self.get_table_def_for_column(stmt, *col_id) {
                let column = stmt.columns.get(*col_id);

                // Check if it's a single-column primary key
                let is_pk = table_def
                    .primary_key
                    .as_ref()
                    .map(|pk| pk.len() == 1 && pk[0] == column.name)
                    .unwrap_or(false);

                // Check if it's a single-column unique constraint
                let is_unique = table_def
                    .unique_constraints
                    .iter()
                    .any(|uc| uc.len() == 1 && uc[0] == column.name);

                if is_pk || is_unique {
                    // IN with 0 or 1 items returns at most one row
                    if *list_len <= 1 {
                        return Some(Cardinality::AtMostOne);
                    }
                    // IN with multiple items could return multiple rows
                    return Some(Cardinality::Unknown);
                }
            }
        }
        None
    }

    /// Analyze IS NULL constraint for cardinality
    fn analyze_is_null_constraint(
        &self,
        stmt: &BoundStatement,
        constraints: &ConstraintInfo,
    ) -> Option<Cardinality> {
        for col_id in &constraints.is_null_constraints {
            if let Some(table_def) = self.get_table_def_for_column(stmt, *col_id) {
                let column = stmt.columns.get(*col_id);

                // Check if this column is part of the primary key
                if let Some(pk_columns) = &table_def.primary_key {
                    if pk_columns.contains(&column.name) {
                        // Primary key cannot be NULL, so result is empty (AtMostOne)
                        return Some(Cardinality::AtMostOne);
                    }
                }
            }
        }
        None
    }

    fn analyze_from_clause(&self, from: &[crate::ir::bound::BoundFromItem]) -> Cardinality {
        if from.is_empty() {
            return Cardinality::ExactlyOne;
        }

        let mut cardinality = Cardinality::Unknown;

        for from_item in from {
            for join in &from_item.joins {
                cardinality =
                    combine_join_cardinality(cardinality, Cardinality::Unknown, join.kind);
            }
        }

        cardinality
    }

    fn analyze_where_condition(
        &self,
        stmt: &BoundStatement,
        where_expr: Option<ExprId>,
    ) -> Option<Cardinality> {
        let expr_id = where_expr?;
        let expr = stmt.exprs.get(expr_id);

        // Check for simple constant false conditions
        match expr {
            BoundExpr::Literal(Value::Boolean(false)) => return Some(Cardinality::AtMostOne),
            BoundExpr::Binary {
                left,
                op: sqlparser::ast::BinaryOperator::Eq,
                right,
            } => {
                let left_expr = stmt.exprs.get(*left);
                let right_expr = stmt.exprs.get(*right);

                if let (BoundExpr::Literal(left_val), BoundExpr::Literal(right_val)) =
                    (left_expr, right_expr)
                {
                    if left_val != right_val {
                        return Some(Cardinality::AtMostOne);
                    }
                }
            },
            _ => {},
        }

        // Extract constraints from WHERE expression
        let mut constraints = ConstraintInfo::default();
        if !self.extract_constraints_from_expr(stmt, expr_id, &mut constraints) {
            return None;
        }

        // Check equality constraints for primary key match
        if !constraints.equality_constraints.is_empty() {
            if let Some(card) = self.check_primary_key_match(stmt, &constraints) {
                return Some(card);
            }
            if let Some(card) = self.check_unique_constraint_match(stmt, &constraints) {
                return Some(card);
            }
        }

        // Check IN constraints
        if !constraints.in_constraints.is_empty() {
            if let Some(card) = self.analyze_in_constraint(stmt, &constraints) {
                return Some(card);
            }
        }

        // Check IS NULL constraints
        if !constraints.is_null_constraints.is_empty() {
            if let Some(card) = self.analyze_is_null_constraint(stmt, &constraints) {
                return Some(card);
            }
        }

        None
    }

    pub(super) fn query_body_cardinality(
        &self,
        stmt: &BoundStatement,
        query: &BoundQueryBody,
    ) -> Cardinality {
        let facts = self.query_body_facts(stmt, query);

        let mut cardinality = match &query.body {
            BoundSetExpr::Select(select) => {
                if !facts.has_from || facts.has_aggregate_without_group_by {
                    Cardinality::ExactlyOne
                } else if facts.has_group_by || facts.has_distinct {
                    Cardinality::Unknown
                } else {
                    self.analyze_from_clause(&select.from)
                }
            },
            BoundSetExpr::Values { .. } => match facts.values_len {
                Some(0) => Cardinality::AtMostOne,
                Some(1) => Cardinality::ExactlyOne,
                Some(_) => Cardinality::AtLeastOne,
                None => Cardinality::Unknown,
            },
            BoundSetExpr::SetOperation {
                op, left, right, ..
            } => {
                let left_card = self.set_expr_cardinality(stmt, left);
                let right_card = self.set_expr_cardinality(stmt, right);

                match op {
                    BoundSetOp::Union => max_cardinality(left_card, right_card),
                    BoundSetOp::Intersect => min_cardinality(left_card, right_card),
                    BoundSetOp::Except => left_card,
                }
            },
            BoundSetExpr::Query(inner) => self.query_body_cardinality(stmt, inner),
        };

        if let BoundSetExpr::Select(select) = &query.body {
            if let Some(where_card) = self.analyze_where_condition(stmt, select.selection) {
                cardinality = min_cardinality(cardinality, where_card);
            }

            if select.having.is_some() {
                cardinality = drop_lower_bound(cardinality);
            }
        }

        if let Some(limit) = facts.limit {
            if limit == 0 {
                cardinality = Cardinality::AtMostOne;
            } else if limit == 1 {
                cardinality = constrain_at_most_one(cardinality);
            }
        }

        if let Some(offset) = facts.offset {
            if offset > 0 {
                cardinality = drop_lower_bound(cardinality);
            }
        }

        cardinality
    }

    fn set_expr_cardinality(&self, stmt: &BoundStatement, expr: &BoundSetExpr) -> Cardinality {
        match expr {
            BoundSetExpr::Select(select) => {
                let facts = QueryFacts {
                    has_from: !select.from.is_empty(),
                    has_aggregate_without_group_by: Self::select_has_aggregate_without_group_by(
                        stmt, select,
                    ),
                    ..Default::default()
                };

                if !facts.has_from || facts.has_aggregate_without_group_by {
                    Cardinality::ExactlyOne
                } else {
                    Cardinality::Unknown
                }
            },
            BoundSetExpr::Values { rows } => match rows.len() {
                0 => Cardinality::AtMostOne,
                1 => Cardinality::ExactlyOne,
                _ => Cardinality::AtLeastOne,
            },
            BoundSetExpr::SetOperation {
                op, left, right, ..
            } => {
                let left_card = self.set_expr_cardinality(stmt, left);
                let right_card = self.set_expr_cardinality(stmt, right);

                match op {
                    BoundSetOp::Union => max_cardinality(left_card, right_card),
                    BoundSetOp::Intersect => min_cardinality(left_card, right_card),
                    BoundSetOp::Except => left_card,
                }
            },
            BoundSetExpr::Query(query) => self.query_body_cardinality(stmt, query),
        }
    }

    fn query_body_facts(&self, stmt: &BoundStatement, query: &BoundQueryBody) -> QueryFacts {
        let mut facts = QueryFacts {
            limit: query
                .limit
                .and_then(|expr_id| Self::literal_u64(stmt, expr_id)),
            offset: query
                .offset
                .and_then(|expr_id| Self::literal_u64(stmt, expr_id)),
            ..Default::default()
        };

        match &query.body {
            BoundSetExpr::Select(select) => {
                facts.has_from = !select.from.is_empty();
                facts.has_group_by = !select.group_by.is_empty();
                facts.has_distinct = select.distinct;
                facts.has_aggregate_without_group_by =
                    Self::select_has_aggregate_without_group_by(stmt, select);
            },
            BoundSetExpr::Values { rows } => {
                facts.values_len = Some(rows.len());
            },
            _ => {},
        }

        facts
    }

    fn select_has_aggregate_without_group_by(stmt: &BoundStatement, select: &BoundSelect) -> bool {
        if !select.group_by.is_empty() {
            return false;
        }

        for proj in &select.projection {
            if Self::expr_has_aggregate(stmt, proj.expr) {
                return true;
            }
        }
        if let Some(having) = select.having {
            if Self::expr_has_aggregate(stmt, having) {
                return true;
            }
        }

        false
    }

    fn literal_u64(stmt: &BoundStatement, expr_id: ExprId) -> Option<u64> {
        match stmt.exprs.get(expr_id) {
            BoundExpr::Literal(Value::Number(text, _)) => text.parse().ok(),
            _ => None,
        }
    }

    fn expr_has_aggregate(stmt: &BoundStatement, expr_id: ExprId) -> bool {
        match stmt.exprs.get(expr_id) {
            BoundExpr::Function {
                function,
                args,
                over,
                ..
            } => {
                let is_aggregate = matches!(function, Function::Aggregate(_)) && !over;
                if is_aggregate {
                    return true;
                }
                args.iter().any(|a| Self::expr_has_aggregate(stmt, *a))
            },
            BoundExpr::Binary { left, right, .. } => {
                Self::expr_has_aggregate(stmt, *left) || Self::expr_has_aggregate(stmt, *right)
            },
            BoundExpr::Unary { expr, .. } | BoundExpr::IsNull { expr, .. } => {
                Self::expr_has_aggregate(stmt, *expr)
            },
            BoundExpr::Case {
                operand,
                conditions,
                results,
                else_result,
            } => {
                operand.iter().any(|e| Self::expr_has_aggregate(stmt, *e))
                    || conditions
                        .iter()
                        .any(|e| Self::expr_has_aggregate(stmt, *e))
                    || results.iter().any(|e| Self::expr_has_aggregate(stmt, *e))
                    || else_result
                        .iter()
                        .any(|e| Self::expr_has_aggregate(stmt, *e))
            },
            BoundExpr::InList { expr, list, .. } => {
                Self::expr_has_aggregate(stmt, *expr)
                    || list.iter().any(|e| Self::expr_has_aggregate(stmt, *e))
            },
            BoundExpr::Column(_)
            | BoundExpr::Literal(_)
            | BoundExpr::Wildcard
            | BoundExpr::Subquery(_)
            | BoundExpr::InSubquery { .. }
            | BoundExpr::Error => false,
        }
    }
}
