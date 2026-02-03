use sqlex_analyzer::Result;
use sqlex_common::DataType;
use sqlparser::ast::{Expr, ObjectName, Value};

use super::{order_by::OrderByExpr, typed::TypedExpr};
use crate::planner::scope::Scope;

/// Aggregate function
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    ArrayAgg,
    StringAgg,
    JsonAgg,
    First,
    Last,
    Custom(String),
}

fn promote_to_large(t: DataType) -> DataType {
    use DataType::*;
    match t {
        TinyInt | SmallInt | Int => BigInt,
        Float => Double,
        other => other,
    }
}

impl AggregateFunction {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_uppercase().as_str() {
            "COUNT" => Some(Self::Count),
            "SUM" => Some(Self::Sum),
            "AVG" => Some(Self::Avg),
            "MIN" => Some(Self::Min),
            "MAX" => Some(Self::Max),
            "ARRAY_AGG" => Some(Self::ArrayAgg),
            "STRING_AGG" => Some(Self::StringAgg),
            "JSON_AGG" => Some(Self::JsonAgg),
            "FIRST_VALUE" | "FIRST" => Some(Self::First),
            "LAST_VALUE" | "LAST" => Some(Self::Last),
            _ => None,
        }
    }

    /// Determine the result type and nullability of this aggregate function.
    pub fn result_type(&self, args: &[TypedExpr]) -> (DataType, bool) {
        let input_type = args
            .first()
            .map(|a| a.data_type.clone())
            .unwrap_or(DataType::Int);

        match self {
            AggregateFunction::Count => (DataType::BigInt, false), // COUNT never returns NULL
            AggregateFunction::Sum => (promote_to_large(input_type), true), // SUM can return NULL for empty set
            AggregateFunction::Avg => (DataType::Double, true),             // AVG can return NULL
            AggregateFunction::Min | AggregateFunction::Max => (input_type, true), // Can return NULL
            AggregateFunction::ArrayAgg => (DataType::Array(Box::new(input_type)), true),
            AggregateFunction::StringAgg => (DataType::Text, true),
            AggregateFunction::JsonAgg => (DataType::Json, true),
            AggregateFunction::First | AggregateFunction::Last => (input_type, true),
            AggregateFunction::Custom(_) => (input_type, true),
        }
    }
}

/// Aggregate expression
#[derive(Debug, Clone)]
pub struct AggregateExpr {
    pub function: AggregateFunction,
    pub args: Vec<TypedExpr>,
    pub distinct: bool,
    pub filter: Option<Box<TypedExpr>>,
    pub order_by: Vec<OrderByExpr>,
}

impl AggregateExpr {
    /// Build AggregateExpr from a Function
    pub fn from_ast(func: &sqlparser::ast::Function, scope: &Scope) -> Result<Self> {
        let name_to_string = |name: &ObjectName| -> String {
            name.0
                .iter()
                .map(|i| i.value.clone())
                .collect::<Vec<_>>()
                .join(".")
        };

        let name = name_to_string(&func.name).to_uppercase();
        let agg_func = if let Some(agg) = AggregateFunction::from_name(&name) {
            agg
        } else {
            AggregateFunction::Custom(name)
        };

        // Parse function arguments
        let mut typed_args = Vec::new();
        if let sqlparser::ast::FunctionArguments::List(ref list) = func.args {
            for arg in &list.args {
                match arg {
                    sqlparser::ast::FunctionArg::Named {
                        arg: sqlparser::ast::FunctionArgExpr::Expr(e),
                        ..
                    } => {
                        typed_args.push(TypedExpr::from_expr(e, scope)?);
                    },
                    sqlparser::ast::FunctionArg::Unnamed(
                        sqlparser::ast::FunctionArgExpr::Expr(e),
                    ) => {
                        typed_args.push(TypedExpr::from_expr(e, scope)?);
                    },
                    sqlparser::ast::FunctionArg::Unnamed(
                        sqlparser::ast::FunctionArgExpr::Wildcard,
                    ) => {
                        // COUNT(*) case - use a dummy expression
                        typed_args.push(TypedExpr::new(
                            Expr::Value(Value::Number("1".to_string(), false)),
                            DataType::Int,
                            false,
                        ));
                    },
                    _ => {},
                }
            }

            // Check for DISTINCT
            let distinct =
                list.duplicate_treatment == Some(sqlparser::ast::DuplicateTreatment::Distinct);

            Ok(AggregateExpr {
                function: agg_func,
                args: typed_args,
                distinct,
                filter: None,
                order_by: Vec::new(),
            })
        } else {
            // No arguments (e.g., COUNT(*))
            Ok(AggregateExpr {
                function: agg_func,
                args: vec![TypedExpr::new(
                    Expr::Value(Value::Number("1".to_string(), false)),
                    DataType::Int,
                    false,
                )],
                distinct: false,
                filter: None,
                order_by: Vec::new(),
            })
        }
    }
}
