use sqlparser::ast::Value;

use crate::{
    analysis::{functions::FunctionKind, infer::Inferrer},
    ir::{
        bound::{BoundExpr, BoundQueryBody, BoundSelect, BoundSetExpr, BoundStatement},
        ids::ExprId,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowCardinality {
    ExactlyOne,
    AtLeastOne,
    AtMostOne,
    Unknown,
}

impl RowCardinality {
    pub(super) fn guarantees_row(self) -> bool {
        matches!(self, Self::ExactlyOne | Self::AtLeastOne)
    }

    fn constrain_at_most_one(self) -> Self {
        match self {
            Self::ExactlyOne => Self::ExactlyOne,
            Self::AtLeastOne => Self::ExactlyOne,
            Self::AtMostOne => Self::AtMostOne,
            Self::Unknown => Self::AtMostOne,
        }
    }

    fn drop_lower_bound(self) -> Self {
        match self {
            Self::ExactlyOne => Self::AtMostOne,
            Self::AtLeastOne => Self::Unknown,
            Self::AtMostOne => Self::AtMostOne,
            Self::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct QueryFacts {
    has_from: bool,
    has_aggregate_without_group_by: bool,
    values_len: Option<usize>,
    limit: Option<u64>,
    offset: Option<u64>,
}

impl Inferrer {
    pub(super) fn query_body_cardinality(
        &self,
        stmt: &BoundStatement,
        query: &BoundQueryBody,
    ) -> RowCardinality {
        let facts = self.query_body_facts(stmt, query);

        let mut cardinality = match &query.body {
            BoundSetExpr::Select(_) => {
                if !facts.has_from || facts.has_aggregate_without_group_by {
                    RowCardinality::ExactlyOne
                } else {
                    RowCardinality::Unknown
                }
            },
            BoundSetExpr::Values { .. } => match facts.values_len {
                Some(0) => RowCardinality::AtMostOne,
                Some(1) => RowCardinality::ExactlyOne,
                Some(_) => RowCardinality::AtLeastOne,
                None => RowCardinality::Unknown,
            },
            BoundSetExpr::SetOperation { .. } => RowCardinality::Unknown,
            BoundSetExpr::Query(inner) => self.query_body_cardinality(stmt, inner),
        };

        if let Some(limit) = facts.limit {
            if limit == 0 {
                cardinality = RowCardinality::AtMostOne;
            } else if limit == 1 {
                cardinality = cardinality.constrain_at_most_one();
            }
        }

        if let Some(offset) = facts.offset {
            if offset > 0 {
                cardinality = cardinality.drop_lower_bound();
            }
        }

        cardinality
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
                kind, args, over, ..
            } => {
                let is_aggregate = matches!(kind, FunctionKind::Aggregate(_)) && !over;
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
            | BoundExpr::Subquery(_)
            | BoundExpr::InSubquery { .. }
            | BoundExpr::Error => false,
        }
    }
}
