use std::convert::TryFrom;

use sqlex_analyzer::error::AnalyzerError;

pub mod aggregation;
pub mod alias;
pub mod cte;
pub mod distinct;
pub mod join;
pub mod limit;
pub mod projection;
pub mod query;
pub mod scan;
pub mod selection;
pub mod set_operation;
pub mod sort;
pub mod values;
pub mod window;

#[derive(Debug, Clone)]
pub enum Relation {
    Query(query::QueryRelation),
    Cte(cte::CteRelation),
    Scan(scan::ScanRelation),
    CteRef(cte::CteRefRelation),
    Values(values::ValuesRelation),
    Selection(selection::SelectionRelation),
    Projection(projection::ProjectionRelation),
    Aggregation(aggregation::AggregationRelation),
    Window(window::WindowRelation),
    Distinct(distinct::DistinctRelation),
    Sort(sort::SortRelation),
    Limit(limit::LimitRelation),
    Alias(alias::AliasRelation),
    Join(join::JoinRelation),
    SetOperation(set_operation::SetOperationRelation),
}

/// Generates conversion implementations between concrete relation structs and `Relation`.
///
/// For each mapping entry (`Variant => ConcreteType`) this macro expands:
/// - `impl From<ConcreteType> for Relation`
/// - `impl TryFrom<&Relation> for &ConcreteType`
///
/// Failed `TryFrom` casts return `AnalyzerError::analysis("A0004", ..)` and report
/// the expected concrete type plus the actual `Relation` variant name extracted from `Debug`.
macro_rules! impl_relation_conversion {
    ($( $variant:ident => $relation_type:path ),+ $(,)?) => {
        $(
            impl From<$relation_type> for Relation {
                fn from(value: $relation_type) -> Self {
                    Self::$variant(value)
                }
            }
        )+

        $(
            impl<'a> TryFrom<&'a Relation> for &'a $relation_type {
                type Error = AnalyzerError;

                fn try_from(relation: &'a Relation) -> Result<Self, Self::Error> {
                    match relation {
                        Relation::$variant(concrete_relation) => Ok(concrete_relation),
                        _ => {
                            let debug_repr = format!("{relation:?}");
                            let actual_variant = debug_repr
                                .split(['(', '{', ' '])
                                .next()
                                .unwrap_or(debug_repr.as_str());
                            Err(AnalyzerError::analysis(
                                "A0004",
                                format!(
                                    "relation cast failed: expected '{}', got 'Relation::{}'",
                                    stringify!($relation_type),
                                    actual_variant
                                ),
                            ))
                        },
                    }
                }
            }
        )+
    };
}

impl_relation_conversion! {
    Query => query::QueryRelation,
    Cte => cte::CteRelation,
    Scan => scan::ScanRelation,
    CteRef => cte::CteRefRelation,
    Values => values::ValuesRelation,
    Selection => selection::SelectionRelation,
    Projection => projection::ProjectionRelation,
    Aggregation => aggregation::AggregationRelation,
    Window => window::WindowRelation,
    Distinct => distinct::DistinctRelation,
    Sort => sort::SortRelation,
    Limit => limit::LimitRelation,
    Alias => alias::AliasRelation,
    Join => join::JoinRelation,
    SetOperation => set_operation::SetOperationRelation,
}
