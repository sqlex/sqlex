use sqlex_common::types::Cardinality;
use sqlparser::ast::Value;

use crate::{
    analysis::{functions::FunctionKind, infer::Inferrer},
    ir::{
        bound::{BoundExpr, BoundQueryBody, BoundSelect, BoundSetExpr, BoundSetOp, BoundStatement},
        ids::ExprId,
    },
};

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
        (Cardinality::ExactlyOne, Cardinality::ExactlyOne) => Cardinality::ExactlyOne,
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

impl Inferrer {
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

        match expr {
            BoundExpr::Literal(Value::Boolean(false)) => Some(Cardinality::AtMostOne),
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

                None
            },
            _ => None,
        }
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
