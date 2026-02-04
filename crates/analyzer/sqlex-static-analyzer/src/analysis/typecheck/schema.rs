use std::collections::HashSet;

use sqlparser::ast;

use crate::{
    analysis::{
        diagnostics::Diagnostic,
        typecheck::{QueryTypeState, TypeContext, infer::merge_types},
    },
    ir::{
        bound::{
            BoundExpr, BoundJoinCondition, BoundJoinKind, BoundQuery, BoundSelect, BoundSetExpr,
            BoundSetOp, BoundTableSource,
        },
        ids::{ColumnId, ExprId, TableId},
        output::{OutputColumn, OutputSchema},
    },
};

impl<'a> TypeContext<'a> {
    pub(super) fn output_schema_for_query(&mut self, query: &BoundQuery) -> OutputSchema {
        let key = query as *const BoundQuery as usize;
        if let Some(schema) = self.schema_cache.get(&key) {
            return schema.clone();
        }

        let mut state = QueryTypeState::new(query);
        let columns = match &query.body {
            BoundSetExpr::Select(select) => {
                self.compute_join_nullability(&mut state, select);
                self.validate_grouping(&state, select);
                self.validate_select_contexts(&state, select);
                self.project_output(&mut state, select)
            },
            BoundSetExpr::SetOperation {
                op, left, right, ..
            } => {
                let left_schema = self.output_schema_for_setexpr(query, left);
                let right_schema = self.output_schema_for_setexpr(query, right);
                self.merge_set_schema(op, left_schema, right_schema)
            },
            BoundSetExpr::Query(subquery) => self.output_schema_for_query(subquery).columns,
            BoundSetExpr::Values { rows } => self.output_values_schema(&mut state, rows),
            BoundSetExpr::Unsupported => {
                self.diagnostics
                    .push(Diagnostic::unsupported_feature("query body in typecheck"));
                Vec::new()
            },
        };

        let schema = OutputSchema { columns };
        self.schema_cache.insert(key, schema.clone());
        schema
    }

    fn output_schema_for_setexpr(
        &mut self,
        query: &BoundQuery,
        expr: &BoundSetExpr,
    ) -> OutputSchema {
        match expr {
            BoundSetExpr::Select(select) => {
                let mut state = QueryTypeState::new(query);
                self.compute_join_nullability(&mut state, select);
                self.validate_grouping(&state, select);
                self.validate_select_contexts(&state, select);
                OutputSchema {
                    columns: self.project_output(&mut state, select),
                }
            },
            BoundSetExpr::SetOperation {
                op, left, right, ..
            } => {
                let left_schema = self.output_schema_for_setexpr(query, left);
                let right_schema = self.output_schema_for_setexpr(query, right);
                OutputSchema {
                    columns: self.merge_set_schema(op, left_schema, right_schema),
                }
            },
            BoundSetExpr::Query(subquery) => self.output_schema_for_query(subquery),
            BoundSetExpr::Values { rows } => OutputSchema {
                columns: self.output_values_schema(&mut QueryTypeState::new(query), rows),
            },
            BoundSetExpr::Unsupported => OutputSchema {
                columns: Vec::new(),
            },
        }
    }

    fn project_output(
        &mut self,
        state: &mut QueryTypeState<'_>,
        select: &BoundSelect,
    ) -> Vec<OutputColumn> {
        let mut cols = Vec::new();
        for (index, proj) in select.projection.iter().enumerate() {
            let type_info = self.infer_expr(state, proj.expr);
            let name = proj
                .alias
                .clone()
                .unwrap_or_else(|| self.infer_expr_name(state, proj.expr, index));
            let lineage = self.collect_lineage(state, proj.expr);

            cols.push(OutputColumn {
                name,
                data_type: type_info.data_type,
                nullability: type_info.nullable,
                lineage,
            });
        }
        cols
    }

    fn output_values_schema(
        &mut self,
        state: &mut QueryTypeState<'_>,
        rows: &[Vec<ExprId>],
    ) -> Vec<OutputColumn> {
        let Some(first) = rows.first() else {
            return Vec::new();
        };

        let col_count = first.len();
        let mut columns = Vec::new();

        for idx in 0..col_count {
            let mut merged_type: Option<sqlex_common::types::DataType> = None;
            let mut nullable = false;
            let mut lineage = Vec::new();

            for row in rows {
                if row.len() != col_count {
                    self.diagnostics
                        .push(Diagnostic::values_column_count_mismatch());
                    break;
                }
                let expr_id = row[idx];
                let info = self.infer_expr(state, expr_id);
                if info.nullable {
                    nullable = true;
                }
                merged_type = merge_types(merged_type, info.data_type.clone());
                let expr_lineage = self.collect_lineage(state, expr_id);
                lineage = if lineage.is_empty() {
                    expr_lineage
                } else {
                    self.merge_lineage(&lineage, &expr_lineage)
                };
            }

            columns.push(OutputColumn {
                name: format!("column{}", idx + 1),
                data_type: merged_type.unwrap_or_else(|| {
                    sqlex_common::types::DataType::Custom("unknown".to_string())
                }),
                nullability: nullable,
                lineage,
            });
        }

        columns
    }

    fn merge_set_schema(
        &mut self,
        _op: &BoundSetOp,
        left: OutputSchema,
        right: OutputSchema,
    ) -> Vec<OutputColumn> {
        if left.columns.len() != right.columns.len() {
            self.diagnostics
                .push(Diagnostic::set_operation_column_count_mismatch());
        }

        let count = left.columns.len().min(right.columns.len());
        let mut columns = Vec::new();
        for idx in 0..count {
            let left_col = &left.columns[idx];
            let right_col = &right.columns[idx];
            let data_type = merge_types(
                Some(left_col.data_type.clone()),
                right_col.data_type.clone(),
            )
            .unwrap_or(left_col.data_type.clone());
            let lineage = self.merge_lineage(&left_col.lineage, &right_col.lineage);
            columns.push(OutputColumn {
                name: left_col.name.clone(),
                data_type,
                nullability: left_col.nullability || right_col.nullability,
                lineage,
            });
        }
        columns
    }

    fn compute_join_nullability(&self, state: &mut QueryTypeState<'_>, select: &BoundSelect) {
        let mut nullable_tables = HashSet::new();

        for from_item in &select.from {
            let mut left_tables = Vec::new();
            left_tables.push(from_item.table);

            for join in &from_item.joins {
                let right_table = join.table;
                match join.kind {
                    BoundJoinKind::Inner | BoundJoinKind::Cross => {},
                    BoundJoinKind::Left => {
                        let preserve_right = self.left_join_preserves_right(
                            state.query,
                            &left_tables,
                            right_table,
                            &join.condition,
                        );
                        if !preserve_right {
                            nullable_tables.insert(right_table);
                        }
                    },
                    BoundJoinKind::Right => {
                        let preserve_left = self.right_join_preserves_left(
                            state.query,
                            &left_tables,
                            right_table,
                            &join.condition,
                        );
                        if !preserve_left {
                            for table in &left_tables {
                                nullable_tables.insert(*table);
                            }
                        }
                    },
                    BoundJoinKind::Full => {
                        for table in &left_tables {
                            nullable_tables.insert(*table);
                        }
                        nullable_tables.insert(right_table);
                    },
                }

                left_tables.push(right_table);
            }
        }

        state.nullable_tables = nullable_tables;
    }

    fn left_join_preserves_right(
        &self,
        query: &BoundQuery,
        left_tables: &[TableId],
        right_table: TableId,
        condition: &Option<BoundJoinCondition>,
    ) -> bool {
        let right_set = HashSet::from([right_table]);
        left_tables
            .iter()
            .any(|left_table| self.fk_guarantees_match(query, *left_table, &right_set, condition))
    }

    fn right_join_preserves_left(
        &self,
        query: &BoundQuery,
        left_tables: &[TableId],
        right_table: TableId,
        condition: &Option<BoundJoinCondition>,
    ) -> bool {
        let left_set: HashSet<TableId> = left_tables.iter().copied().collect();
        self.fk_guarantees_match(query, right_table, &left_set, condition)
    }

    fn fk_guarantees_match(
        &self,
        query: &BoundQuery,
        fk_table: TableId,
        ref_tables: &HashSet<TableId>,
        condition: &Option<BoundJoinCondition>,
    ) -> bool {
        let Some(condition) = condition else {
            return false;
        };
        let Some((col_a, col_b)) = self.extract_join_columns(query, condition) else {
            return false;
        };

        let col_a_table = query.columns.get(col_a).table;
        let col_b_table = query.columns.get(col_b).table;

        let (fk_col, ref_col, ref_table) =
            if col_a_table == fk_table && ref_tables.contains(&col_b_table) {
                (col_a, col_b, col_b_table)
            } else if col_b_table == fk_table && ref_tables.contains(&col_a_table) {
                (col_b, col_a, col_a_table)
            } else {
                return false;
            };

        let fk_table_name = match &query.tables.get(fk_table).source {
            BoundTableSource::Table { name } => name,
            _ => return false,
        };
        let ref_table_name = match &query.tables.get(ref_table).source {
            BoundTableSource::Table { name } => name,
            _ => return false,
        };

        let fk_col_name = query.columns.get(fk_col).name.clone();
        let ref_col_name = query.columns.get(ref_col).name.clone();

        let fk_table_def = match self.catalog.get_table(fk_table_name) {
            Some(def) => def,
            None => return false,
        };

        let fk_col_def = match fk_table_def.get_column(&fk_col_name) {
            Some(def) => def,
            None => return false,
        };
        if fk_col_def.nullable {
            return false;
        }

        fk_table_def.foreign_keys.iter().any(|fk| {
            fk.ref_table == *ref_table_name
                && fk.columns.len() == 1
                && fk.ref_columns.len() == 1
                && fk.columns[0] == fk_col_name
                && fk.ref_columns[0] == ref_col_name
        })
    }

    fn extract_join_columns(
        &self,
        query: &BoundQuery,
        condition: &BoundJoinCondition,
    ) -> Option<(ColumnId, ColumnId)> {
        match condition {
            BoundJoinCondition::On(expr_id) => match query.exprs.get(*expr_id) {
                BoundExpr::Binary { left, op, right } if *op == ast::BinaryOperator::Eq => {
                    match (query.exprs.get(*left), query.exprs.get(*right)) {
                        (BoundExpr::Column(left_col), BoundExpr::Column(right_col)) => {
                            Some((*left_col, *right_col))
                        },
                        _ => None,
                    }
                },
                _ => None,
            },
            _ => None,
        }
    }

    fn infer_expr_name(&self, state: &QueryTypeState<'_>, expr_id: ExprId, index: usize) -> String {
        match state.query.exprs.get(expr_id) {
            crate::ir::bound::BoundExpr::Column(column_id) => {
                let column = state.query.columns.get(*column_id);
                column.name.clone()
            },
            _ => format!("col_{}", index),
        }
    }
}
