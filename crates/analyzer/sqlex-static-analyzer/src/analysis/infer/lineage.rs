use std::collections::HashSet;

use crate::{
    analysis::infer::{Inferrer, QueryTypeState},
    ir::{
        bound::{BoundExpr, BoundTableSource},
        ids::{ColumnId, ExprId},
        output::LineageColumn,
    },
};

impl<'a> Inferrer<'a> {
    pub(super) fn collect_lineage(
        &mut self,
        state: &QueryTypeState<'_>,
        expr_id: ExprId,
    ) -> Vec<LineageColumn> {
        let mut set = HashSet::new();
        self.collect_lineage_inner(state, expr_id, &mut set);
        let mut lineage: Vec<_> = set.into_iter().collect();
        sort_lineage(&mut lineage);
        lineage
    }

    fn collect_lineage_inner(
        &mut self,
        state: &QueryTypeState<'_>,
        expr_id: ExprId,
        out: &mut HashSet<LineageColumn>,
    ) {
        match state.query.exprs.get(expr_id) {
            BoundExpr::Column(column_id) => {
                for col in self.resolve_column_lineage(state, *column_id) {
                    out.insert(col);
                }
            },
            BoundExpr::Binary { left, right, .. } => {
                self.collect_lineage_inner(state, *left, out);
                self.collect_lineage_inner(state, *right, out);
            },
            BoundExpr::Unary { expr, .. } => {
                self.collect_lineage_inner(state, *expr, out);
            },
            BoundExpr::IsNull { expr, .. } => {
                self.collect_lineage_inner(state, *expr, out);
            },
            BoundExpr::Function { args, .. } => {
                for arg in args {
                    self.collect_lineage_inner(state, *arg, out);
                }
            },
            BoundExpr::Case {
                operand,
                conditions,
                results,
                else_result,
            } => {
                if let Some(expr_id) = operand {
                    self.collect_lineage_inner(state, *expr_id, out);
                }
                for expr_id in conditions {
                    self.collect_lineage_inner(state, *expr_id, out);
                }
                for expr_id in results {
                    self.collect_lineage_inner(state, *expr_id, out);
                }
                if let Some(expr_id) = else_result {
                    self.collect_lineage_inner(state, *expr_id, out);
                }
            },
            BoundExpr::Subquery(subquery) => {
                let schema = self.output_schema_for_query(subquery);
                if let Some(col) = schema.columns.first() {
                    for lineage in &col.lineage {
                        out.insert(lineage.clone());
                    }
                }
            },
            _ => {},
        }
    }

    fn resolve_column_lineage(
        &mut self,
        state: &QueryTypeState<'_>,
        column_id: ColumnId,
    ) -> Vec<LineageColumn> {
        let column = state.query.columns.get(column_id);
        let table = state.query.tables.get(column.table);

        match &table.source {
            BoundTableSource::Table { name } => vec![LineageColumn {
                table: Some(name.clone()),
                column: column.name.clone(),
            }],
            BoundTableSource::Derived { query } => {
                let schema = self.output_schema_for_query(query);
                schema
                    .columns
                    .iter()
                    .find(|c| c.name == column.name)
                    .map(|c| c.lineage.clone())
                    .unwrap_or_default()
            },
            BoundTableSource::Cte { name } => {
                let cte = state.query.ctes.iter().rev().find(|cte| cte.name == *name);
                if let Some(cte) = cte {
                    let schema = self.output_schema_for_query(&cte.query);
                    schema
                        .columns
                        .iter()
                        .find(|c| c.name == column.name)
                        .map(|c| c.lineage.clone())
                        .unwrap_or_default()
                } else {
                    Vec::new()
                }
            },
        }
    }

    pub(super) fn merge_lineage(
        &self,
        left: &[LineageColumn],
        right: &[LineageColumn],
    ) -> Vec<LineageColumn> {
        let mut set = HashSet::new();
        for col in left {
            set.insert(col.clone());
        }
        for col in right {
            set.insert(col.clone());
        }
        let mut lineage: Vec<_> = set.into_iter().collect();
        sort_lineage(&mut lineage);
        lineage
    }
}

fn sort_lineage(lineage: &mut [LineageColumn]) {
    lineage.sort_by(|a, b| {
        let key_a = (a.table.as_deref().unwrap_or(""), a.column.as_str());
        let key_b = (b.table.as_deref().unwrap_or(""), b.column.as_str());
        key_a.cmp(&key_b)
    });
}
