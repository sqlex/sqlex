use sqlparser::ast::Value;

use crate::{
    analysis::infer::{Inferrer, QueryTypeState},
    ir::{
        bound::{BoundExpr, BoundQuery, BoundSelect, BoundSetExpr},
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

impl<'a> Inferrer<'a> {
    pub(super) fn query_cardinality(&self, query: &BoundQuery) -> RowCardinality {
        let facts = self.query_facts(query);

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
            BoundSetExpr::Query(inner) => self.query_cardinality(inner),
            BoundSetExpr::Unsupported => RowCardinality::Unknown,
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

    fn query_facts(&self, query: &BoundQuery) -> QueryFacts {
        let mut facts = QueryFacts {
            limit: query
                .limit
                .and_then(|expr_id| self.literal_u64(query, expr_id)),
            offset: query
                .offset
                .and_then(|expr_id| self.literal_u64(query, expr_id)),
            ..Default::default()
        };

        match &query.body {
            BoundSetExpr::Select(select) => {
                facts.has_from = !select.from.is_empty();
                facts.has_aggregate_without_group_by =
                    self.select_has_aggregate_without_group_by(query, select);
            },
            BoundSetExpr::Values { rows } => {
                facts.values_len = Some(rows.len());
            },
            _ => {},
        }

        facts
    }

    fn select_has_aggregate_without_group_by(
        &self,
        query: &BoundQuery,
        select: &BoundSelect,
    ) -> bool {
        if !select.group_by.is_empty() {
            return false;
        }

        let state = QueryTypeState::new(query);
        for proj in &select.projection {
            if self.analyze_group_expr(&state, proj.expr).has_aggregate {
                return true;
            }
        }
        if let Some(having) = select.having {
            if self.analyze_group_expr(&state, having).has_aggregate {
                return true;
            }
        }

        false
    }

    fn literal_u64(&self, query: &BoundQuery, expr_id: ExprId) -> Option<u64> {
        match query.exprs.get(expr_id) {
            BoundExpr::Literal(Value::Number(text, _)) => text.parse().ok(),
            _ => None,
        }
    }
}
