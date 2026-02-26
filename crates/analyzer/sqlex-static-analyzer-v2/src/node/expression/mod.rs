use std::convert::TryFrom;

use sqlex_analyzer::error::AnalyzerError;

pub mod aggregate_call;
pub mod binary_op;
pub mod case;
pub mod cast;
pub mod column_ref;
pub mod exists;
pub mod function;
pub mod in_list;
pub mod in_subquery;
pub mod is_null;
pub mod literal;
pub mod placeholder;
pub mod scalar_subquery;
pub mod unary_op;
pub mod window_call;

#[derive(Debug, Clone)]
pub enum Expression {
    ColumnRef(column_ref::ColumnRef),
    Literal(literal::LiteralExpression),
    BinaryOp(binary_op::BinaryOpExpression),
    UnaryOp(unary_op::UnaryOpExpression),
    Function(function::FunctionExpression),
    AggregateCall(aggregate_call::AggregateCallExpression),
    WindowCall(window_call::WindowCallExpression),
    Cast(cast::CastExpression),
    IsNull(is_null::IsNullExpression),
    Case(case::CaseExpression),
    InList(in_list::InListExpression),
    InSubquery(in_subquery::InSubqueryExpression),
    Exists(exists::ExistsExpression),
    ScalarSubquery(scalar_subquery::ScalarSubqueryExpression),
    Placeholder(placeholder::PlaceholderExpression),
}

/// Generates conversion implementations between concrete expression structs and `Expression`.
///
/// For each mapping entry (`Variant => ConcreteType`) this macro expands:
/// - `impl From<ConcreteType> for Expression`
/// - `impl TryFrom<&Expression> for &ConcreteType`
///
/// Failed `TryFrom` casts return `AnalyzerError::analysis("A0004", ..)` and report
/// the expected concrete type plus the actual `Expression` variant name extracted from `Debug`.
macro_rules! impl_expression_conversion {
    ($( $variant:ident => $expression_type:path ),+ $(,)?) => {
        $(
            impl From<$expression_type> for Expression {
                fn from(value: $expression_type) -> Self {
                    Self::$variant(value)
                }
            }
        )+

        $(
            impl<'a> TryFrom<&'a Expression> for &'a $expression_type {
                type Error = AnalyzerError;

                fn try_from(expression: &'a Expression) -> Result<Self, Self::Error> {
                    match expression {
                        Expression::$variant(concrete_expression) => Ok(concrete_expression),
                        _ => {
                            let debug_repr = format!("{expression:?}");
                            let actual_variant = debug_repr
                                .split(['(', '{', ' '])
                                .next()
                                .unwrap_or(debug_repr.as_str());
                            Err(AnalyzerError::analysis(
                                "A0004",
                                format!(
                                    "expression cast failed: expected '{}', got 'Expression::{}'",
                                    stringify!($expression_type),
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

impl_expression_conversion! {
    ColumnRef => column_ref::ColumnRef,
    Literal => literal::LiteralExpression,
    BinaryOp => binary_op::BinaryOpExpression,
    UnaryOp => unary_op::UnaryOpExpression,
    Function => function::FunctionExpression,
    AggregateCall => aggregate_call::AggregateCallExpression,
    WindowCall => window_call::WindowCallExpression,
    Cast => cast::CastExpression,
    IsNull => is_null::IsNullExpression,
    Case => case::CaseExpression,
    InList => in_list::InListExpression,
    InSubquery => in_subquery::InSubqueryExpression,
    Exists => exists::ExistsExpression,
    ScalarSubquery => scalar_subquery::ScalarSubqueryExpression,
    Placeholder => placeholder::PlaceholderExpression,
}
