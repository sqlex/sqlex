use sqlex_analyzer::{AnalyzerError, Result};
use sqlex_common::DataType;
use sqlparser::ast::{
    Expr, Function, FunctionArg, FunctionArgExpr, FunctionArguments, ObjectName, Value,
};

use crate::planner::{scope::Scope, types};

/// Helper to extract table name from object name
fn name_to_string(name: &ObjectName) -> String {
    name.0
        .iter()
        .map(|i| i.value.clone())
        .collect::<Vec<String>>()
        .join(".")
}

/// Expression with inferred type information
#[derive(Debug, Clone)]
pub struct TypedExpr {
    pub expr: Expr,
    pub data_type: DataType,
    pub nullable: bool,
}

impl TypedExpr {
    /// Create a new typed expression
    /// Create a new typed expression
    pub fn new(expr: Expr, data_type: DataType, nullable: bool) -> Self {
        Self {
            expr,
            data_type,
            nullable,
        }
    }

    /// Build a TypedExpr from an AST Expr
    pub fn from_expr(expr: &Expr, scope: &Scope) -> Result<Self> {
        let (data_type, nullable) = match expr {
            Expr::Identifier(ident) => {
                let col = scope.resolve_column(None, &ident.value)?;
                (col.data_type, col.nullable)
            },
            Expr::CompoundIdentifier(idents) => {
                if idents.len() == 2 {
                    let col = scope.resolve_column(Some(&idents[0].value), &idents[1].value)?;
                    (col.data_type, col.nullable)
                } else {
                    return Err(AnalyzerError::AnalysisError(
                        "Deep compound identifiers not supported".to_string(),
                    ));
                }
            },
            Expr::Value(v) => {
                let dt = Self::infer_value_type(v);
                let nullable = matches!(v, Value::Null);
                (dt, nullable)
            },
            Expr::BinaryOp { left, op, right } => {
                let l = Self::from_expr(left, scope)?;
                let r = Self::from_expr(right, scope)?;
                let dt = types::binary_op_type(l.data_type, op.clone(), r.data_type);
                let nullable = l.nullable || r.nullable;
                (dt, nullable)
            },
            Expr::UnaryOp { op, expr } => {
                let e = Self::from_expr(expr, scope)?;
                let dt = types::unary_op_type(*op, e.data_type);
                (dt, e.nullable)
            },
            Expr::Function(func) => {
                // Check if it's NULLIF - NULLIF always returns nullable
                let name_upper = name_to_string(&func.name).to_uppercase();
                if name_upper == "NULLIF" {
                    let (dt, _) = Self::infer_function_type(func, scope)?;
                    // NULLIF is always nullable because it returns NULL if args are equal
                    (dt, true)
                } else {
                    Self::infer_function_type(func, scope)?
                }
            },
            Expr::Case {
                operand: _,
                conditions: _,
                results,
                else_result,
            } => {
                // CASE expression type and nullability
                // Type: use first result branch
                let first_result = results.first().ok_or(AnalyzerError::AnalysisError(
                    "CASE expression has no THEN branches".to_string(),
                ))?;
                let first_typed = Self::from_expr(first_result, scope)?;
                let data_type = first_typed.data_type;

                // Nullability: use helper
                let mut when_nullabilities = Vec::new();
                for result_expr in results {
                    let typed = Self::from_expr(result_expr, scope)?;
                    when_nullabilities.push(typed.nullable);
                }

                let mut else_nullable = None;
                if let Some(else_expr) = else_result {
                    let typed = Self::from_expr(else_expr, scope)?;
                    else_nullable = Some(typed.nullable);
                }

                // If no ELSE, implicitly NULL, so nullable
                let nullable = if else_result.is_none() {
                    true
                } else {
                    // Check if any WHEN branch is nullable
                    if when_nullabilities.iter().any(|&n| n) {
                        true
                    } else {
                        // Check if ELSE branch is nullable
                        else_nullable.unwrap_or(false)
                    }
                };

                (data_type, nullable)
            },
            Expr::Nested(e) => {
                let t = Self::from_expr(e, scope)?;
                (t.data_type, t.nullable)
            },
            _ => (DataType::Custom("unknown".to_string()), true),
        };
        Ok(TypedExpr::new(expr.clone(), data_type, nullable))
    }

    fn infer_value_type(v: &Value) -> DataType {
        match v {
            Value::Number(num, _) => {
                // Check if it's an integer or float
                if num.contains('.') || num.contains('e') || num.contains('E') {
                    DataType::Double
                } else {
                    DataType::Int
                }
            },
            Value::SingleQuotedString(_) | Value::DoubleQuotedString(_) => DataType::Text,
            Value::Boolean(_) => DataType::Bool,
            Value::Null => DataType::Custom("NULL".to_string()),
            _ => DataType::Text,
        }
    }

    fn infer_function_type(func: &Function, scope: &Scope) -> Result<(DataType, bool)> {
        let name = name_to_string(&func.name);

        let args_vec = if matches!(func.args, FunctionArguments::None) {
            Vec::new()
        } else if let FunctionArguments::List(ref list) = func.args {
            list.args.clone()
        } else {
            return Err(AnalyzerError::AnalysisError(
                "Subquery as function argument not supported".to_string(),
            ));
        };

        let mut typed_args = Vec::new();
        for arg in &args_vec {
            match arg {
                FunctionArg::Named {
                    arg: FunctionArgExpr::Expr(e),
                    ..
                } => {
                    typed_args.push(Self::from_expr(e, scope)?);
                },
                FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) => {
                    typed_args.push(Self::from_expr(e, scope)?);
                },
                _ => {},
            }
        }

        let arg_types: Vec<DataType> = typed_args.iter().map(|a| a.data_type.clone()).collect();

        // Try to infer aggregate function types first
        let upper_name = name.to_uppercase();
        let return_type = match upper_name.as_str() {
            "COUNT" => types::aggregate_return_type(&AggregateFunction::Count, DataType::Int),
            "SUM" => {
                let input_type = arg_types.first().cloned().unwrap_or(DataType::Int);
                types::aggregate_return_type(&AggregateFunction::Sum, input_type)
            },
            "AVG" => {
                let input_type = arg_types.first().cloned().unwrap_or(DataType::Int);
                types::aggregate_return_type(&AggregateFunction::Avg, input_type)
            },
            "MIN" => {
                let input_type = arg_types.first().cloned().unwrap_or(DataType::Int);
                types::aggregate_return_type(&AggregateFunction::Min, input_type)
            },
            "MAX" => {
                let input_type = arg_types.first().cloned().unwrap_or(DataType::Int);
                types::aggregate_return_type(&AggregateFunction::Max, input_type)
            },
            _ => {
                // Fall back to regular function type inference
                types::function_return_type(&name, &arg_types)
                    .unwrap_or(DataType::Custom("unknown".to_string()))
            },
        };

        let mut nullable = typed_args.iter().any(|a| a.nullable);

        match upper_name.as_str() {
            "COALESCE" => {
                // COALESCE is nullable only if ALL arguments are nullable
                nullable = typed_args.iter().all(|a| a.nullable);
            },
            "SUM" | "AVG" | "MIN" | "MAX" | "LEAD" | "LAG" | "FIRST_VALUE" | "LAST_VALUE"
            | "NTH_VALUE" => {
                // Aggregates/window functions usually return nullable
                nullable = true;
            },
            "COUNT" | "ROW_NUMBER" | "RANK" | "DENSE_RANK" => {
                nullable = false;
            },
            _ => {},
        }

        Ok((return_type, nullable))
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

/// ORDER BY expression
#[derive(Debug, Clone)]
pub struct OrderByExpr {
    pub expr: TypedExpr,
    pub asc: bool,
    pub nulls_first: Option<bool>,
}

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

/// Determine the result type and nullability of an aggregate function.
pub fn aggregate_result_type(func: &AggregateFunction, args: &[TypedExpr]) -> (DataType, bool) {
    let input_type = args
        .first()
        .map(|a| a.data_type.clone())
        .unwrap_or(DataType::Int);

    match func {
        AggregateFunction::Count => (DataType::BigInt, false), // COUNT never returns NULL
        AggregateFunction::Sum => (input_type, true),          // SUM can return NULL for empty set
        AggregateFunction::Avg => (DataType::Double, true),    // AVG can return NULL
        AggregateFunction::Min | AggregateFunction::Max => (input_type, true), // Can return NULL
        AggregateFunction::ArrayAgg => (DataType::Array(Box::new(input_type)), true),
        AggregateFunction::StringAgg => (DataType::Text, true),
        AggregateFunction::JsonAgg => (DataType::Json, true),
        AggregateFunction::First | AggregateFunction::Last => (input_type, true),
        AggregateFunction::Custom(_) => (input_type, true),
    }
}
