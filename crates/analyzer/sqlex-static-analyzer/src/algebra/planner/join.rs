use std::collections::HashSet;

use sqlparser::ast::{Expr, Join, JoinConstraint, JoinOperator};

use crate::{
    algebra::{
        expr::{RelExpr, SelectionNode},
        planner::{
            Algebraizer,
            context::{BuildContext, RelationScope},
        },
        scalar::{BoundBinaryOp, BoundColumn, BoundScalarExpr, ColumnOrigin, OutputSchema},
    },
    catalog::{model::Catalog, normalize::normalize_object_name},
    diagnostics::{Diagnostic, Phase},
    functions::registry::FunctionRegistry,
};

impl Algebraizer {
    pub(crate) fn build_join(
        &self,
        left_expr: RelExpr,
        scopes: &mut Vec<RelationScope>,
        join: &Join,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &mut BuildContext,
    ) -> Result<RelExpr, Diagnostic> {
        if join.global {
            return Err(Diagnostic::todo(Phase::Algebraize, "GLOBAL JOIN planning"));
        }

        let (right_expr, right_scope) =
            self.build_table_factor(&join.relation, catalog, functions, context)?;
        let (kind, on_expr) = self.join_kind_and_condition(&join.join_operator)?;
        let using_columns = self.extract_join_using_columns(&join.join_operator);
        let using_set = using_columns.iter().cloned().collect::<HashSet<String>>();

        let mut visible_right_scope = right_scope.clone();
        if !using_set.is_empty() {
            visible_right_scope
                .schema
                .columns
                .retain(|column| !using_set.contains(&column.name));
        }

        let mut join_scopes = scopes.clone();
        join_scopes.push(right_scope.clone());
        context.relation_scopes = join_scopes;

        let left_schema = super::output_schema_of(&left_expr)?;
        let right_schema = super::output_schema_of(&right_expr)?;
        let mut effective_kind = kind.clone();
        let mut bound_condition = None;
        if let Some(on_expr) = on_expr {
            let (condition, _) = self.bind_expr(on_expr, catalog, functions, context)?;
            if self.outer_join_is_effectively_inner(
                kind,
                &condition,
                &left_schema,
                &right_schema,
                catalog,
            ) {
                effective_kind = crate::algebra::expr::JoinKind::Inner;
            }
            bound_condition = Some(condition);
        } else if !using_columns.is_empty() {
            let condition =
                self.build_join_using_condition(&using_columns, &left_schema, &right_schema)?;
            if self.outer_join_is_effectively_inner(
                kind,
                &condition,
                &left_schema,
                &right_schema,
                catalog,
            ) {
                effective_kind = crate::algebra::expr::JoinKind::Inner;
            }
            bound_condition = Some(condition);
        }

        let mut left_columns = left_schema.columns.clone();
        let mut right_columns = right_schema.columns.clone();
        if !using_set.is_empty() {
            right_columns.retain(|column| !using_set.contains(&column.name));
        }
        match effective_kind {
            crate::algebra::expr::JoinKind::Left => {
                for column in &mut right_columns {
                    column.nullable = true;
                }
            },
            crate::algebra::expr::JoinKind::Right => {
                for column in &mut left_columns {
                    column.nullable = true;
                }
            },
            crate::algebra::expr::JoinKind::Full => {
                for column in &mut left_columns {
                    column.nullable = true;
                }
                for column in &mut right_columns {
                    column.nullable = true;
                }
            },
            crate::algebra::expr::JoinKind::Inner | crate::algebra::expr::JoinKind::Cross => {},
        }

        let mut columns = left_columns;
        columns.extend(right_columns);
        let join_schema = OutputSchema {
            relation_id: context.allocate_relation_id(),
            columns,
        };

        let mut join_expr = RelExpr::Join(crate::algebra::expr::JoinNode {
            left: Box::new(left_expr),
            right: Box::new(right_expr),
            kind: effective_kind,
            schema: join_schema.clone(),
        });

        if let Some(condition) = bound_condition {
            join_expr = RelExpr::Selection(SelectionNode {
                input: Box::new(join_expr),
                condition,
                schema: join_schema.clone(),
            });
        }

        scopes.push(visible_right_scope);
        context.relation_scopes = scopes.clone();
        Ok(join_expr)
    }

    fn join_kind_and_condition<'a>(
        &self,
        operator: &'a JoinOperator,
    ) -> Result<(crate::algebra::expr::JoinKind, Option<&'a Expr>), Diagnostic> {
        match operator {
            JoinOperator::Inner(constraint) => {
                self.join_constraint_with_kind(crate::algebra::expr::JoinKind::Inner, constraint)
            },
            JoinOperator::LeftOuter(constraint) => {
                self.join_constraint_with_kind(crate::algebra::expr::JoinKind::Left, constraint)
            },
            JoinOperator::RightOuter(constraint) => {
                self.join_constraint_with_kind(crate::algebra::expr::JoinKind::Right, constraint)
            },
            JoinOperator::FullOuter(constraint) => {
                if matches!(self.dialect, sqlex_common::dialect::Dialect::MySQL) {
                    return Err(Diagnostic::todo(
                        Phase::Algebraize,
                        "FULL JOIN planning for mysql",
                    ));
                }
                self.join_constraint_with_kind(crate::algebra::expr::JoinKind::Full, constraint)
            },
            JoinOperator::CrossJoin => Ok((crate::algebra::expr::JoinKind::Cross, None)),
            _ => Err(Diagnostic::todo(
                Phase::Algebraize,
                "this JOIN operator planning",
            )),
        }
    }

    fn join_constraint_with_kind<'a>(
        &self,
        kind: crate::algebra::expr::JoinKind,
        constraint: &'a JoinConstraint,
    ) -> Result<(crate::algebra::expr::JoinKind, Option<&'a Expr>), Diagnostic> {
        match constraint {
            JoinConstraint::On(expr) => Ok((kind, Some(expr))),
            JoinConstraint::None => Ok((kind, None)),
            JoinConstraint::Using(_) => Ok((kind, None)),
            JoinConstraint::Natural => Err(Diagnostic::todo(
                Phase::Algebraize,
                "JOIN USING/NATURAL planning",
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

    fn build_join_using_condition(
        &self,
        using_columns: &[String],
        left_schema: &OutputSchema,
        right_schema: &OutputSchema,
    ) -> Result<BoundScalarExpr, Diagnostic> {
        let mut condition = None;
        for column_name in using_columns {
            let left_slot = self.resolve_join_using_slot(left_schema, column_name)?;
            let right_slot = self.resolve_join_using_slot(right_schema, column_name)?;
            let equality = BoundScalarExpr::BinaryOp {
                left: Box::new(BoundScalarExpr::SlotRef(left_slot)),
                op: BoundBinaryOp::Eq,
                right: Box::new(BoundScalarExpr::SlotRef(right_slot)),
            };

            condition = Some(match condition {
                Some(existing) => BoundScalarExpr::BinaryOp {
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

    fn resolve_join_using_slot(
        &self,
        schema: &OutputSchema,
        column_name: &str,
    ) -> Result<u32, Diagnostic> {
        let mut slots = schema
            .columns
            .iter()
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
        kind: crate::algebra::expr::JoinKind,
        condition: &BoundScalarExpr,
        left_schema: &OutputSchema,
        right_schema: &OutputSchema,
        catalog: &Catalog,
    ) -> bool {
        let (preserved_columns, other_columns) = match kind {
            crate::algebra::expr::JoinKind::Left => (&left_schema.columns, &right_schema.columns),
            crate::algebra::expr::JoinKind::Right => (&right_schema.columns, &left_schema.columns),
            crate::algebra::expr::JoinKind::Inner
            | crate::algebra::expr::JoinKind::Cross
            | crate::algebra::expr::JoinKind::Full => return false,
        };

        let mut slot_pairs = Vec::new();
        collect_equality_slot_pairs(condition, &mut slot_pairs);
        slot_pairs.into_iter().any(|(left_slot, right_slot)| {
            self.fk_slot_pair_guarantees_match(
                left_slot,
                right_slot,
                preserved_columns,
                other_columns,
                catalog,
            ) || self.fk_slot_pair_guarantees_match(
                right_slot,
                left_slot,
                preserved_columns,
                other_columns,
                catalog,
            )
        })
    }

    fn fk_slot_pair_guarantees_match(
        &self,
        preserved_slot: u32,
        other_slot: u32,
        preserved_columns: &[BoundColumn],
        other_columns: &[BoundColumn],
        catalog: &Catalog,
    ) -> bool {
        let Some(preserved_column) = preserved_columns
            .iter()
            .find(|column| column.slot_id == preserved_slot)
        else {
            return false;
        };
        let Some(other_column) = other_columns
            .iter()
            .find(|column| column.slot_id == other_slot)
        else {
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

        let Some(table_schema) = catalog.table(preserved_table) else {
            return false;
        };

        table_schema.foreign_keys.iter().any(|foreign_key| {
            foreign_key.columns.len() == 1
                && foreign_key.ref_columns.len() == 1
                && foreign_key.columns[0] == *preserved_column_name
                && foreign_key.ref_table == *other_table
                && foreign_key.ref_columns[0] == *other_column_name
        })
    }
}

fn collect_equality_slot_pairs(expr: &BoundScalarExpr, output: &mut Vec<(u32, u32)>) {
    match expr {
        BoundScalarExpr::BinaryOp {
            left,
            op: BoundBinaryOp::And,
            right,
        } => {
            collect_equality_slot_pairs(left, output);
            collect_equality_slot_pairs(right, output);
        },
        BoundScalarExpr::BinaryOp {
            left,
            op: BoundBinaryOp::Eq,
            right,
        } => {
            if let (BoundScalarExpr::SlotRef(left_slot), BoundScalarExpr::SlotRef(right_slot)) =
                (&**left, &**right)
            {
                output.push((*left_slot, *right_slot));
            }
        },
        _ => {},
    }
}
