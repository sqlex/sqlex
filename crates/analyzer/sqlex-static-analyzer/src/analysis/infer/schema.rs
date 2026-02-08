use sqlex_analyzer::extension::DataTypeExt;
use sqlex_common::types::DataType;

use crate::{
    analysis::{
        diagnostics::Diagnostic,
        infer::{Inferrer, QueryTypeState, SchemaCacheKey, expr_formatter::ExprFormatter},
    },
    ir::{
        bound::{
            BoundExpr, BoundJoinCondition, BoundJoinKind, BoundQueryBody, BoundSelect,
            BoundSetExpr, BoundSetOp, BoundStatement,
        },
        ids::{ColumnId, ExprId, TableId},
        output::{OutputColumn, OutputSchema},
    },
};

impl Inferrer<'_> {
    // ------------------------------------------------------------------
    // Entry points
    // ------------------------------------------------------------------

    pub(super) fn output_schema_for_statement(&mut self, stmt: &BoundStatement) -> OutputSchema {
        if let Some(schema) = self.schema_cache.get(&SchemaCacheKey::TopLevel) {
            return schema.clone();
        }
        let mut state = QueryTypeState::new(stmt);
        let columns = self.infer_set_expr(&mut state, &stmt.query.body);
        let cardinality = self.query_body_cardinality(stmt, &stmt.query);
        let schema = OutputSchema {
            columns,
            cardinality,
        };
        self.schema_cache
            .insert(SchemaCacheKey::TopLevel, schema.clone());
        schema
    }

    pub(super) fn output_schema_for_query_body(
        &mut self,
        state: &mut QueryTypeState<'_>,
        query: &BoundQueryBody,
    ) -> OutputSchema {
        let columns = self.infer_set_expr(state, &query.body);
        let cardinality = self.query_body_cardinality(state.stmt, query);
        OutputSchema {
            columns,
            cardinality,
        }
    }

    // ------------------------------------------------------------------
    // Set expression dispatch
    // ------------------------------------------------------------------

    fn infer_set_expr(
        &mut self,
        state: &mut QueryTypeState<'_>,
        body: &BoundSetExpr,
    ) -> Vec<OutputColumn> {
        match body {
            BoundSetExpr::Select(select) => {
                self.compute_join_nullability(state, select);
                self.project_output(state, select)
            },
            BoundSetExpr::SetOperation {
                op, left, right, ..
            } => {
                let left_cols = self.infer_set_expr(state, left);
                let right_cols = self.infer_set_expr(state, right);
                self.merge_set_schema(op, left_cols, right_cols)
            },
            BoundSetExpr::Query(inner) => self.infer_set_expr(state, &inner.body),
            BoundSetExpr::Values { rows } => self.output_values_schema(state, rows),
        }
    }

    // ------------------------------------------------------------------
    // SELECT projection
    // ------------------------------------------------------------------

    fn project_output(
        &mut self,
        state: &mut QueryTypeState<'_>,
        select: &BoundSelect,
    ) -> Vec<OutputColumn> {
        select
            .projection
            .iter()
            .map(|proj| {
                let info = self.infer_expr(state, proj.expr);
                let name = proj
                    .alias
                    .clone()
                    .unwrap_or_else(|| self.infer_expr_name(state, proj.expr));
                OutputColumn {
                    name,
                    data_type: info.data_type,
                    nullability: info.nullable,
                }
            })
            .collect()
    }

    // ------------------------------------------------------------------
    // VALUES schema
    // ------------------------------------------------------------------

    fn output_values_schema(
        &mut self,
        state: &mut QueryTypeState<'_>,
        rows: &[Vec<ExprId>],
    ) -> Vec<OutputColumn> {
        let Some(first_row) = rows.first() else {
            return Vec::new();
        };

        let col_count = first_row.len();

        // Seed with the first row.
        let mut types: Vec<Option<DataType>> = Vec::with_capacity(col_count);
        let mut nullables: Vec<bool> = Vec::with_capacity(col_count);

        for expr_id in first_row {
            let info = self.infer_expr(state, *expr_id);
            types.push(Some(info.data_type));
            nullables.push(info.nullable);
        }

        // Merge remaining rows.
        for row in rows.iter().skip(1) {
            if row.len() != col_count {
                self.diagnostics
                    .push(Diagnostic::values_column_count_mismatch());
                break;
            }
            for (i, expr_id) in row.iter().enumerate() {
                let info = self.infer_expr(state, *expr_id);
                types[i] =
                    DataType::merge_common_type(self.dialect, types[i].take(), &info.data_type);
                if info.nullable {
                    nullables[i] = true;
                }
            }
        }

        types
            .into_iter()
            .zip(nullables)
            .enumerate()
            .map(|(i, (dt, nullable))| OutputColumn {
                name: format!("column{}", i + 1),
                data_type: dt.unwrap_or_else(|| DataType::Custom("unknown".to_string())),
                nullability: nullable,
            })
            .collect()
    }

    // ------------------------------------------------------------------
    // UNION / INTERSECT / EXCEPT
    // ------------------------------------------------------------------

    fn merge_set_schema(
        &mut self,
        _op: &BoundSetOp,
        left: Vec<OutputColumn>,
        right: Vec<OutputColumn>,
    ) -> Vec<OutputColumn> {
        if left.len() != right.len() {
            self.diagnostics
                .push(Diagnostic::set_operation_column_count_mismatch());
            return left;
        }

        left.into_iter()
            .zip(right)
            .map(|(l, r)| {
                let data_type =
                    DataType::merge_common_type(self.dialect, Some(l.data_type), &r.data_type)
                        .unwrap_or_else(|| DataType::Custom("unknown".to_string()));
                OutputColumn {
                    name: l.name,
                    data_type,
                    nullability: l.nullability || r.nullability,
                }
            })
            .collect()
    }

    // ------------------------------------------------------------------
    // JOIN nullability
    // ------------------------------------------------------------------

    fn compute_join_nullability(&self, state: &mut QueryTypeState<'_>, select: &BoundSelect) {
        for from_item in &select.from {
            for join in &from_item.joins {
                match join.kind {
                    BoundJoinKind::Left => {
                        // Right side becomes nullable unless FK guarantees a match.
                        if !self.left_join_preserves_right(state, join.table, &join.condition) {
                            state.nullable_tables.insert(join.table);
                        }
                    },
                    BoundJoinKind::Right => {
                        // Left side (the driving table) becomes nullable.
                        if !self.right_join_preserves_left(state, from_item.table, &join.condition)
                        {
                            state.nullable_tables.insert(from_item.table);
                        }
                    },
                    BoundJoinKind::Full => {
                        state.nullable_tables.insert(from_item.table);
                        state.nullable_tables.insert(join.table);
                    },
                    BoundJoinKind::Inner | BoundJoinKind::Cross => {
                        // No additional nullability.
                    },
                }
            }
        }
    }

    fn left_join_preserves_right(
        &self,
        state: &QueryTypeState<'_>,
        right_table: TableId,
        condition: &Option<BoundJoinCondition>,
    ) -> bool {
        let Some(condition) = condition else {
            return false;
        };
        let Some((left_col, right_col)) = Self::extract_join_columns(state.stmt, condition) else {
            return false;
        };
        let right_column = state.stmt.columns.get(right_col);
        if right_column.table != right_table {
            return false;
        }
        self.fk_guarantees_match(state, left_col, right_col)
    }

    fn right_join_preserves_left(
        &self,
        state: &QueryTypeState<'_>,
        left_table: TableId,
        condition: &Option<BoundJoinCondition>,
    ) -> bool {
        let Some(condition) = condition else {
            return false;
        };
        let Some((left_col, right_col)) = Self::extract_join_columns(state.stmt, condition) else {
            return false;
        };
        let left_column = state.stmt.columns.get(left_col);
        if left_column.table != left_table {
            return false;
        }
        self.fk_guarantees_match(state, right_col, left_col)
    }

    /// Check if a foreign key constraint guarantees that a join will match.
    /// Returns true if:
    /// 1. The foreign key column is NOT NULL
    /// 2. There exists a foreign key constraint from fk_col to pk_col
    fn fk_guarantees_match(
        &self,
        state: &QueryTypeState<'_>,
        fk_col: ColumnId,
        pk_col: ColumnId,
    ) -> bool {
        let fk_column = state.stmt.columns.get(fk_col);
        let pk_column = state.stmt.columns.get(pk_col);

        // Get table names
        let fk_table = state.stmt.tables.get(fk_column.table);
        let pk_table = state.stmt.tables.get(pk_column.table);

        let fk_table_name = match &fk_table.source {
            crate::ir::bound::BoundTableSource::Table { name } => name,
            _ => return false,
        };

        let pk_table_name = match &pk_table.source {
            crate::ir::bound::BoundTableSource::Table { name } => name,
            _ => return false,
        };

        // Check if fk_col is NOT NULL
        let fk_col_nullable = fk_column.nullable.unwrap_or(true);
        if fk_col_nullable {
            return false;
        }

        // Look up the table definition in catalog
        let Some(fk_table_def) = self.catalog.get_table(fk_table_name) else {
            return false;
        };

        // Check if there's a foreign key constraint from fk_col to pk_col
        for fk in &fk_table_def.foreign_keys {
            // Check if this FK references the right table
            if !fk.ref_table.eq_ignore_ascii_case(pk_table_name) {
                continue;
            }

            // Check if the FK columns match
            // For simplicity, we only handle single-column FKs
            if fk.columns.len() == 1 && fk.ref_columns.len() == 1 {
                let fk_col_name = &fk.columns[0];
                let ref_col_name = &fk.ref_columns[0];

                if fk_col_name.eq_ignore_ascii_case(&fk_column.name)
                    && ref_col_name.eq_ignore_ascii_case(&pk_column.name)
                {
                    return true;
                }
            }
        }

        false
    }

    fn extract_join_columns(
        stmt: &BoundStatement,
        condition: &BoundJoinCondition,
    ) -> Option<(ColumnId, ColumnId)> {
        match condition {
            BoundJoinCondition::On(expr_id) => {
                let expr = stmt.exprs.get(*expr_id);
                if let BoundExpr::Binary {
                    left,
                    op: sqlparser::ast::BinaryOperator::Eq,
                    right,
                } = expr
                {
                    let left_expr = stmt.exprs.get(*left);
                    let right_expr = stmt.exprs.get(*right);
                    if let (BoundExpr::Column(l), BoundExpr::Column(r)) = (left_expr, right_expr) {
                        return Some((*l, *r));
                    }
                }
                None
            },
            BoundJoinCondition::Using(_) | BoundJoinCondition::Natural => None,
        }
    }

    // ------------------------------------------------------------------
    // Expression name inference
    // ------------------------------------------------------------------

    fn infer_expr_name(&self, state: &QueryTypeState<'_>, expr_id: ExprId) -> String {
        let formatter = ExprFormatter::new(state.stmt, self.dialect);
        formatter.format_expr(expr_id, 0)
    }
}
