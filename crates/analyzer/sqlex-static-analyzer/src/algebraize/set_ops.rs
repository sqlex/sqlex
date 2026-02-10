use sqlparser::ast::{SetExpr, SetOperator, SetQuantifier};

use super::Algebraizer;
use crate::{
    diagnostics::Diagnostic,
    ir::{auxiliary::SetOp, relational::RelationalExpr, scalar::ScalarExpr},
};

impl<'a> Algebraizer<'a> {
    pub(super) fn algebraize_set_expr(&mut self, set_expr: &SetExpr) -> Option<RelationalExpr> {
        match set_expr {
            SetExpr::Select(select) => self.algebraize_select(select),
            SetExpr::Query(query) => self.algebraize_query(query),
            SetExpr::SetOperation {
                op,
                left,
                right,
                set_quantifier,
            } => {
                let left_expr = self.algebraize_set_expr(left)?;
                let right_expr = self.algebraize_set_expr(right)?;
                Some(RelationalExpr::SetOperation {
                    left: Box::new(left_expr),
                    right: Box::new(right_expr),
                    op: map_set_op(op),
                    all: matches!(
                        set_quantifier,
                        SetQuantifier::All | SetQuantifier::AllByName
                    ),
                })
            },
            SetExpr::Values(values) => {
                let mut rows = Vec::new();
                for row in &values.rows {
                    let bound_row: Vec<ScalarExpr> =
                        row.iter().map(|e| self.build_scalar_expr(e)).collect();
                    rows.push(bound_row);
                }
                self.validate_values(&rows);
                Some(RelationalExpr::Values { rows })
            },
            _ => {
                self.diagnostics.push(Diagnostic::unsupported_feature(
                    "set expression in algebraizer",
                ));
                None
            },
        }
    }

    fn validate_values(&mut self, rows: &[Vec<ScalarExpr>]) {
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
}

fn map_set_op(op: &SetOperator) -> SetOp {
    match op {
        SetOperator::Union => SetOp::Union,
        SetOperator::Intersect => SetOp::Intersect,
        SetOperator::Except => SetOp::Except,
        _ => SetOp::Union,
    }
}
