use sqlex_common::dialect::Dialect;

use crate::{
    analysis::diagnostics::Diagnostic,
    ir::bound::{BoundQueryBody, BoundSetExpr, BoundStatement},
};

mod context;
mod grouping;

pub struct ValidationResult {
    pub diagnostics: Vec<Diagnostic>,
}

pub(crate) struct Validator {
    dialect: Dialect,
    diagnostics: Vec<Diagnostic>,
}

impl Validator {
    pub(crate) fn new(dialect: Dialect) -> Self {
        Self {
            dialect,
            diagnostics: Vec::new(),
        }
    }

    pub(crate) fn validate(mut self, stmt: &BoundStatement) -> ValidationResult {
        self.validate_query_body(stmt, &stmt.query);
        ValidationResult {
            diagnostics: self.diagnostics,
        }
    }

    fn validate_query_body(&mut self, stmt: &BoundStatement, query: &BoundQueryBody) {
        self.validate_set_expr(stmt, &query.body);
    }

    fn validate_set_expr(&mut self, stmt: &BoundStatement, expr: &BoundSetExpr) {
        match expr {
            BoundSetExpr::Select(select) => {
                self.validate_grouping(stmt, select);
                self.validate_select_contexts(stmt, select);
            },
            BoundSetExpr::SetOperation { left, right, .. } => {
                self.validate_set_expr(stmt, left);
                self.validate_set_expr(stmt, right);
            },
            BoundSetExpr::Query(subquery) => {
                self.validate_query_body(stmt, subquery);
            },
            BoundSetExpr::Values { rows } => {
                self.validate_values(rows);
            },
        }

        // Validate nested subqueries in expressions
        self.validate_nested_subqueries(stmt, expr);
    }

    fn validate_values(&mut self, rows: &[Vec<crate::ir::ids::ExprId>]) {
        if let Some(first) = rows.first() {
            let expected = first.len();
            for row in rows.iter().skip(1) {
                if row.len() != expected {
                    self.diagnostics
                        .push(Diagnostic::values_column_count_mismatch());
                    break;
                }
            }
        }
    }

    fn validate_nested_subqueries(&mut self, stmt: &BoundStatement, expr: &BoundSetExpr) {
        if let BoundSetExpr::Select(select) = expr {
            for proj in &select.projection {
                self.validate_expr_subqueries(stmt, proj.expr);
            }
            if let Some(sel) = select.selection {
                self.validate_expr_subqueries(stmt, sel);
            }
            if let Some(having) = select.having {
                self.validate_expr_subqueries(stmt, having);
            }
            for from_item in &select.from {
                let table = stmt.tables.get(from_item.table);
                if let crate::ir::bound::BoundTableSource::Derived { query } = &table.source {
                    self.validate_query_body(stmt, query);
                }
                for join in &from_item.joins {
                    let join_table = stmt.tables.get(join.table);
                    if let crate::ir::bound::BoundTableSource::Derived { query } =
                        &join_table.source
                    {
                        self.validate_query_body(stmt, query);
                    }
                }
            }
        }

        // Validate CTE subqueries
        for cte in &stmt.ctes {
            self.validate_query_body(stmt, &cte.query);
        }
    }

    fn validate_expr_subqueries(&mut self, stmt: &BoundStatement, expr_id: crate::ir::ids::ExprId) {
        match stmt.exprs.get(expr_id) {
            crate::ir::bound::BoundExpr::Subquery(query) => {
                self.validate_query_body(stmt, query);
            },
            crate::ir::bound::BoundExpr::InSubquery { subquery, .. } => {
                self.validate_query_body(stmt, subquery);
            },
            _ => {},
        }
    }
}
