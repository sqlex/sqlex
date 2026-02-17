use std::collections::{HashMap, HashSet};

use sqlex_common::dialect::Dialect;
use sqlparser::ast::{Expr, Join, JoinConstraint, JoinOperator};

use crate::{
    algebraizer::{
        Algebraizer,
        model::{
            expression::{BoundBinaryOp, Expression},
            relation::{JoinKind, Relation, SelectionNode},
            schema::{BoundColumn, ColumnOrigin, OutputSchema},
        },
        scope::RelationBinding,
    },
    catalog::{model::TableSchema, normalize::normalize_object_name},
    diagnostics::{Diagnostic, Phase},
};

#[derive(Debug, Clone)]
struct JoinUsingPair {
    column_name: String,
    left_slot: u32,
    right_slot: u32,
}

impl Algebraizer<'_> {
    pub(crate) fn build_join_relation(
        &mut self,
        left_relation: Relation,
        scopes: &mut Vec<RelationBinding>,
        join: &Join,
    ) -> Result<Relation, Diagnostic> {
        if join.global {
            return Err(Diagnostic::new(
                "A3053",
                Phase::Algebraize,
                "GLOBAL JOIN is not supported in this algebraizer path",
            ));
        }

        let (right_relation, right_scope) = self.build_table_factor_relation(&join.relation)?;
        let (kind, on_expr) = self.join_kind_and_condition(&join.join_operator)?;
        let using_columns = self.extract_join_using_columns(&join.join_operator);

        let mut join_scopes = scopes.clone();
        join_scopes.push(right_scope.clone());
        self.relation_scope.set_current(join_scopes.clone());

        let left_schema = left_relation.output_schema();
        let right_schema = right_relation.output_schema();

        let using_pairs = self.resolve_join_using_pairs(
            &using_columns,
            scopes,
            &right_scope,
            left_schema,
            right_schema,
        )?;

        let mut effective_kind = kind.clone();
        let mut bound_condition = None;
        if let Some(on_expr) = on_expr {
            let (condition, _) = self.build_expression(on_expr)?;
            if self.outer_join_is_effectively_inner(
                kind.clone(),
                &condition,
                left_schema,
                right_schema,
            ) {
                effective_kind = JoinKind::Inner;
            }
            bound_condition = Some(condition);
        } else if !using_pairs.is_empty() {
            let condition = self.build_join_using_condition(&using_pairs)?;
            if self.outer_join_is_effectively_inner(
                kind.clone(),
                &condition,
                left_schema,
                right_schema,
            ) {
                effective_kind = JoinKind::Inner;
            }
            bound_condition = Some(condition);
        }

        let mut left_columns = left_schema.columns.clone();
        let mut right_columns = right_schema.columns.clone();
        force_outer_join_nullability(
            effective_kind.clone(),
            &mut left_columns,
            &mut right_columns,
        );

        let left_slot_columns: HashMap<u32, BoundColumn> = left_columns
            .iter()
            .cloned()
            .map(|column| (column.slot_id, column))
            .collect();
        let right_slot_columns: HashMap<u32, BoundColumn> = right_columns
            .iter()
            .cloned()
            .map(|column| (column.slot_id, column))
            .collect();

        let mut merged_columns = Vec::new();
        let mut hidden_slots = HashSet::new();
        for pair in &using_pairs {
            let left_column = left_slot_columns.get(&pair.left_slot).ok_or_else(|| {
                Diagnostic::new(
                    "A3057",
                    Phase::Algebraize,
                    format!(
                        "internal algebraizer invariant violated: missing left USING slot {}",
                        pair.left_slot
                    ),
                )
            })?;
            let right_column = right_slot_columns.get(&pair.right_slot).ok_or_else(|| {
                Diagnostic::new(
                    "A3057",
                    Phase::Algebraize,
                    format!(
                        "internal algebraizer invariant violated: missing right USING slot {}",
                        pair.right_slot
                    ),
                )
            })?;

            merged_columns.push(BoundColumn {
                slot_id: self.allocate_slot_id(),
                name: pair.column_name.clone(),
                table_alias: None,
                data_type: left_column
                    .data_type
                    .clone()
                    .or_else(|| right_column.data_type.clone()),
                nullable: merged_using_nullability(
                    effective_kind.clone(),
                    left_column.nullable,
                    right_column.nullable,
                ),
                origin: ColumnOrigin::Derived,
            });

            hidden_slots.insert(pair.left_slot);
            hidden_slots.insert(pair.right_slot);
        }

        let mut columns = left_columns;
        columns.extend(right_columns);
        columns.extend(merged_columns.clone());
        let join_schema = OutputSchema {
            relation_id: self.allocate_relation_id(),
            columns,
        };

        let mut join_relation = Relation::Join(crate::algebraizer::model::relation::JoinNode {
            left: Box::new(left_relation),
            right: Box::new(right_relation),
            kind: effective_kind.clone(),
            schema: join_schema.clone(),
        });

        if let Some(condition) = bound_condition {
            join_relation = Relation::Selection(SelectionNode {
                input: Box::new(join_relation),
                condition,
                schema: join_schema.clone(),
            });
        }

        let mut updated_scopes = scopes.clone();
        let mut updated_right_scope = right_scope;
        if !hidden_slots.is_empty() {
            for scope in &mut updated_scopes {
                hide_unqualified_slots(scope, &hidden_slots);
            }
            hide_unqualified_slots(&mut updated_right_scope, &hidden_slots);
        }
        updated_scopes.push(updated_right_scope);
        if !merged_columns.is_empty() {
            updated_scopes.push(RelationBinding {
                qualifier_names: Vec::new(),
                schema: OutputSchema {
                    relation_id: self.allocate_relation_id(),
                    columns: merged_columns,
                },
                hidden_unqualified_slot_ids: HashSet::new(),
            });
        }

        *scopes = updated_scopes.clone();
        self.relation_scope.set_current(updated_scopes);
        Ok(join_relation)
    }

    fn join_kind_and_condition<'a>(
        &self,
        operator: &'a JoinOperator,
    ) -> Result<(JoinKind, Option<&'a Expr>), Diagnostic> {
        match operator {
            JoinOperator::Inner(constraint) => {
                self.join_constraint_with_kind(JoinKind::Inner, constraint)
            },
            JoinOperator::LeftOuter(constraint) => {
                self.join_constraint_with_kind(JoinKind::Left, constraint)
            },
            JoinOperator::RightOuter(constraint) => {
                self.join_constraint_with_kind(JoinKind::Right, constraint)
            },
            JoinOperator::FullOuter(constraint) => {
                if matches!(self.dialect, Dialect::MySQL) {
                    return Err(Diagnostic::new(
                        "A3054",
                        Phase::Algebraize,
                        "FULL JOIN is not supported for mysql",
                    ));
                }
                self.join_constraint_with_kind(JoinKind::Full, constraint)
            },
            JoinOperator::CrossJoin => Ok((JoinKind::Cross, None)),
            _ => Err(Diagnostic::new(
                "A3055",
                Phase::Algebraize,
                format!(
                    "JOIN operator is not supported for dialect {}: {:?}",
                    self.dialect, operator
                ),
            )),
        }
    }

    fn join_constraint_with_kind<'a>(
        &self,
        kind: JoinKind,
        constraint: &'a JoinConstraint,
    ) -> Result<(JoinKind, Option<&'a Expr>), Diagnostic> {
        match constraint {
            JoinConstraint::On(expr) => Ok((kind, Some(expr))),
            JoinConstraint::None => Ok((kind, None)),
            JoinConstraint::Using(_) => Ok((kind, None)),
            JoinConstraint::Natural => Err(Diagnostic::new(
                "A3056",
                Phase::Algebraize,
                "NATURAL JOIN is not supported in this algebraizer path",
            )),
        }
    }

    fn extract_join_using_columns(&self, operator: &JoinOperator) -> Vec<String> {
        let constraint = match operator {
            JoinOperator::Inner(constraint)
            | JoinOperator::LeftOuter(constraint)
            | JoinOperator::RightOuter(constraint)
            | JoinOperator::FullOuter(constraint) => constraint,
            _ => return Vec::new(),
        };

        let JoinConstraint::Using(columns) = constraint else {
            return Vec::new();
        };

        columns
            .iter()
            .map(|column| normalize_object_name(column, self.dialect))
            .collect()
    }

    fn resolve_join_using_pairs(
        &self,
        using_columns: &[String],
        left_scopes: &[RelationBinding],
        right_scope: &RelationBinding,
        left_schema: &OutputSchema,
        right_schema: &OutputSchema,
    ) -> Result<Vec<JoinUsingPair>, Diagnostic> {
        let mut pairs = Vec::with_capacity(using_columns.len());
        for column_name in using_columns {
            let left_slot = self.resolve_join_using_slot_in_scopes(left_scopes, column_name)?;
            let right_slot = self.resolve_join_using_slot_in_scopes(
                std::slice::from_ref(right_scope),
                column_name,
            )?;

            if find_column_by_slot(left_schema, left_slot).is_none() {
                return Err(Diagnostic::new(
                    "A3057",
                    Phase::Algebraize,
                    format!(
                        "internal algebraizer invariant violated: left slot {} not found in schema",
                        left_slot
                    ),
                ));
            }
            if find_column_by_slot(right_schema, right_slot).is_none() {
                return Err(Diagnostic::new(
                    "A3057",
                    Phase::Algebraize,
                    format!(
                        "internal algebraizer invariant violated: right slot {} not found in schema",
                        right_slot
                    ),
                ));
            }

            pairs.push(JoinUsingPair {
                column_name: column_name.clone(),
                left_slot,
                right_slot,
            });
        }
        Ok(pairs)
    }

    fn build_join_using_condition(
        &self,
        using_pairs: &[JoinUsingPair],
    ) -> Result<Expression, Diagnostic> {
        let mut condition = None;
        for pair in using_pairs {
            let equality = Expression::BinaryOp {
                left: Box::new(Expression::SlotRef(pair.left_slot)),
                op: BoundBinaryOp::Eq,
                right: Box::new(Expression::SlotRef(pair.right_slot)),
            };

            condition = Some(match condition {
                Some(existing) => Expression::BinaryOp {
                    left: Box::new(existing),
                    op: BoundBinaryOp::And,
                    right: Box::new(equality),
                },
                None => equality,
            });
        }

        condition.ok_or_else(|| {
            Diagnostic::new(
                "A3027",
                Phase::Algebraize,
                "JOIN USING requires at least one shared column",
            )
        })
    }

    fn resolve_join_using_slot_in_scopes(
        &self,
        scopes: &[RelationBinding],
        column_name: &str,
    ) -> Result<u32, Diagnostic> {
        let mut slots = scopes
            .iter()
            .flat_map(|scope| {
                scope.schema.columns.iter().filter(move |column| {
                    !scope.hidden_unqualified_slot_ids.contains(&column.slot_id)
                })
            })
            .filter(|column| column.name == column_name)
            .map(|column| column.slot_id);

        let Some(slot_id) = slots.next() else {
            return Err(Diagnostic::new(
                "A3008",
                Phase::Algebraize,
                format!("column not found: {column_name}"),
            ));
        };
        if slots.next().is_some() {
            return Err(Diagnostic::new(
                "A3009",
                Phase::Algebraize,
                format!("ambiguous column reference: {column_name}"),
            ));
        }
        Ok(slot_id)
    }

    fn outer_join_is_effectively_inner(
        &self,
        kind: JoinKind,
        condition: &Expression,
        left_schema: &OutputSchema,
        right_schema: &OutputSchema,
    ) -> bool {
        match kind {
            JoinKind::Left => self.guaranteed_match_from_preserved_side(
                condition,
                &left_schema.columns,
                &right_schema.columns,
            ),
            JoinKind::Right => self.guaranteed_match_from_preserved_side(
                condition,
                &right_schema.columns,
                &left_schema.columns,
            ),
            JoinKind::Inner | JoinKind::Cross | JoinKind::Full => false,
        }
    }

    fn guaranteed_match_from_preserved_side(
        &self,
        condition: &Expression,
        preserved_columns: &[BoundColumn],
        other_columns: &[BoundColumn],
    ) -> bool {
        let mut slot_pairs = Vec::new();
        if !collect_pure_equijoin_slot_pairs(condition, &mut slot_pairs) || slot_pairs.is_empty() {
            return false;
        }

        self.slot_pairs_cover_full_fk_to_unique(preserved_columns, other_columns, &slot_pairs)
    }

    fn slot_pairs_cover_full_fk_to_unique(
        &self,
        preserved_columns: &[BoundColumn],
        other_columns: &[BoundColumn],
        slot_pairs: &[(u32, u32)],
    ) -> bool {
        let preserved_slot_map: HashMap<u32, &BoundColumn> = preserved_columns
            .iter()
            .map(|column| (column.slot_id, column))
            .collect();
        let other_slot_map: HashMap<u32, &BoundColumn> = other_columns
            .iter()
            .map(|column| (column.slot_id, column))
            .collect();

        let mut mapping: HashMap<String, String> = HashMap::new();
        let mut preserved_table_name: Option<String> = None;
        let mut other_table_name: Option<String> = None;

        for (left_slot, right_slot) in slot_pairs {
            let (preserved_column, other_column) = if let (Some(preserved), Some(other)) = (
                preserved_slot_map.get(left_slot),
                other_slot_map.get(right_slot),
            ) {
                (*preserved, *other)
            } else if let (Some(preserved), Some(other)) = (
                preserved_slot_map.get(right_slot),
                other_slot_map.get(left_slot),
            ) {
                (*preserved, *other)
            } else {
                return false;
            };

            if preserved_column.nullable {
                return false;
            }

            let ColumnOrigin::Base {
                table: preserved_table,
                column: preserved_column_name,
            } = &preserved_column.origin
            else {
                return false;
            };
            let ColumnOrigin::Base {
                table: other_table,
                column: other_column_name,
            } = &other_column.origin
            else {
                return false;
            };

            match &preserved_table_name {
                Some(existing) if existing != preserved_table => return false,
                Some(_) => {},
                None => preserved_table_name = Some(preserved_table.clone()),
            }
            match &other_table_name {
                Some(existing) if existing != other_table => return false,
                Some(_) => {},
                None => other_table_name = Some(other_table.clone()),
            }

            match mapping.get(preserved_column_name) {
                Some(existing) if existing != other_column_name => return false,
                Some(_) => {},
                None => {
                    mapping.insert(preserved_column_name.clone(), other_column_name.clone());
                },
            }
        }

        let Some(preserved_table_name) = preserved_table_name else {
            return false;
        };
        let Some(other_table_name) = other_table_name else {
            return false;
        };

        let Some(preserved_table_schema) = self.catalog.table(&preserved_table_name) else {
            return false;
        };
        let Some(other_table_schema) = self.catalog.table(&other_table_name) else {
            return false;
        };

        preserved_table_schema
            .foreign_keys
            .iter()
            .any(|foreign_key| {
                if foreign_key.ref_table != other_table_name {
                    return false;
                }
                if foreign_key.columns.is_empty()
                    || foreign_key.columns.len() != foreign_key.ref_columns.len()
                {
                    return false;
                }
                if mapping.len() != foreign_key.columns.len() {
                    return false;
                }
                if !foreign_key_columns_are_not_null(preserved_table_schema, &foreign_key.columns) {
                    return false;
                }
                if !foreign_key
                    .columns
                    .iter()
                    .zip(foreign_key.ref_columns.iter())
                    .all(|(foreign_key_column, referenced_column)| {
                        mapping
                            .get(foreign_key_column)
                            .is_some_and(|mapped| mapped == referenced_column)
                    })
                {
                    return false;
                }

                columns_cover_unique_key(other_table_schema, &foreign_key.ref_columns)
            })
    }
}

fn force_outer_join_nullability(
    kind: JoinKind,
    left_columns: &mut [BoundColumn],
    right_columns: &mut [BoundColumn],
) {
    match kind {
        JoinKind::Left => {
            for column in right_columns {
                column.nullable = true;
            }
        },
        JoinKind::Right => {
            for column in left_columns {
                column.nullable = true;
            }
        },
        JoinKind::Full => {
            for column in left_columns.iter_mut() {
                column.nullable = true;
            }
            for column in right_columns.iter_mut() {
                column.nullable = true;
            }
        },
        JoinKind::Inner | JoinKind::Cross => {},
    }
}

fn merged_using_nullability(kind: JoinKind, left_nullable: bool, right_nullable: bool) -> bool {
    match kind {
        JoinKind::Left => left_nullable,
        JoinKind::Right => right_nullable,
        JoinKind::Inner | JoinKind::Full | JoinKind::Cross => left_nullable || right_nullable,
    }
}

fn hide_unqualified_slots(scope: &mut RelationBinding, hidden_slots: &HashSet<u32>) {
    for column in &scope.schema.columns {
        if hidden_slots.contains(&column.slot_id) {
            scope.hidden_unqualified_slot_ids.insert(column.slot_id);
        }
    }
}

fn find_column_by_slot(schema: &OutputSchema, slot_id: u32) -> Option<&BoundColumn> {
    schema
        .columns
        .iter()
        .find(|column| column.slot_id == slot_id)
}

fn collect_pure_equijoin_slot_pairs(expr: &Expression, output: &mut Vec<(u32, u32)>) -> bool {
    match expr {
        Expression::BinaryOp {
            left,
            op: BoundBinaryOp::And,
            right,
        } => {
            collect_pure_equijoin_slot_pairs(left, output)
                && collect_pure_equijoin_slot_pairs(right, output)
        },
        Expression::BinaryOp {
            left,
            op: BoundBinaryOp::Eq,
            right,
        } => {
            if let (Expression::SlotRef(left_slot), Expression::SlotRef(right_slot)) =
                (&**left, &**right)
            {
                output.push((*left_slot, *right_slot));
                true
            } else {
                false
            }
        },
        _ => false,
    }
}

fn foreign_key_columns_are_not_null(table: &TableSchema, columns: &[String]) -> bool {
    columns.iter().all(|column_name| {
        table
            .columns
            .iter()
            .find(|column| column.name == *column_name)
            .is_some_and(|column| !column.nullable)
    })
}

fn columns_cover_unique_key(table: &TableSchema, columns: &[String]) -> bool {
    table
        .primary_key
        .as_ref()
        .is_some_and(|primary_key| same_column_set(&primary_key.columns, columns))
        || table
            .unique_keys
            .iter()
            .any(|key| same_column_set(&key.columns, columns))
}

fn same_column_set(left: &[String], right: &[String]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let left_set = left.iter().map(String::as_str).collect::<HashSet<_>>();
    let right_set = right.iter().map(String::as_str).collect::<HashSet<_>>();
    left_set.len() == left.len() && right_set.len() == right.len() && left_set == right_set
}
