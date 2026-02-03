use sqlex_analyzer::{AnalyzerError, Result};
use sqlex_common::DataType;
use sqlparser::ast::{
    BinaryOperator, Expr, Function, FunctionArg, FunctionArgExpr, FunctionArguments, ObjectName,
    UnaryOperator, Value,
};

use super::{aggregate::AggregateFunction, scalar::ScalarFunction, window::WindowFunction};
use crate::planner::scope::Scope;

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
                let dt = binary_op_type(l.data_type, op.clone(), r.data_type);
                let nullable = l.nullable || r.nullable;
                (dt, nullable)
            },
            Expr::UnaryOp { op, expr } => {
                let e = Self::from_expr(expr, scope)?;
                let dt = unary_op_type(*op, e.data_type);
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

        // 1. Try to infer aggregate function types
        if let Some(agg) = AggregateFunction::from_name(&name) {
            return Ok(agg.result_type(&typed_args));
        }

        // 2. Try to infer window function types
        if let Some(win) = WindowFunction::from_name(&name) {
            return Ok(win.result_type(&typed_args));
        }

        // 3. Try to infer scalar function types
        if let Some(scalar) = ScalarFunction::from_name(&name) {
            return Ok(scalar.result_type(&typed_args));
        }

        // 4. Fallback
        Ok((DataType::Custom("unknown".to_string()), true))
    }
}

/// Infer type for binary operation
fn binary_op_type(left: DataType, op: BinaryOperator, right: DataType) -> DataType {
    match op {
        // Arithmetic operations: promote to larger type
        BinaryOperator::Plus
        | BinaryOperator::Minus
        | BinaryOperator::Multiply
        | BinaryOperator::Modulo => promote_numeric(left, right),

        // Division usually returns float
        BinaryOperator::Divide => DataType::Double,

        // Comparison operations: return boolean
        BinaryOperator::Gt
        | BinaryOperator::Lt
        | BinaryOperator::GtEq
        | BinaryOperator::LtEq
        | BinaryOperator::Eq
        | BinaryOperator::NotEq => DataType::Bool,

        // Logical operations
        BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Xor => DataType::Bool,

        // String concatenation
        BinaryOperator::StringConcat => DataType::Text,

        // Bitwise operations: return integer
        BinaryOperator::BitwiseOr
        | BinaryOperator::BitwiseAnd
        | BinaryOperator::BitwiseXor
        | BinaryOperator::PGBitwiseShiftLeft
        | BinaryOperator::PGBitwiseShiftRight => promote_numeric(left, right),

        // Other operators: default to left operand type
        _ => left,
    }
}

/// Infer type for unary operation
fn unary_op_type(op: UnaryOperator, operand: DataType) -> DataType {
    match op {
        UnaryOperator::Not => DataType::Bool,
        UnaryOperator::Plus | UnaryOperator::Minus => operand,
        _ => operand,
    }
}

/// Promote numeric types to a common type
fn promote_numeric(a: DataType, b: DataType) -> DataType {
    use DataType::*;

    match (&a, &b) {
        // If either is Double, result is Double
        (Double, _) | (_, Double) => Double,

        // If either is Float, result is Float (unless the other is Double)
        (Float, _) | (_, Float) => Float,

        // If either is Decimal, result is Decimal
        (Decimal, _) | (_, Decimal) => Decimal,

        // If either is BigInt, result is BigInt
        (BigInt, _) | (_, BigInt) => BigInt,

        // If either is Int, result is Int
        (Int, _) | (_, Int) => Int,

        // If either is SmallInt, result is SmallInt
        (SmallInt, _) | (_, SmallInt) => SmallInt,

        // TinyInt stays TinyInt
        (TinyInt, TinyInt) => TinyInt,

        // Default to the first type
        _ => a,
    }
}
