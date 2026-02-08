use sqlex_common::dialect::Dialect;
use sqlparser::ast::{BinaryOperator, UnaryOperator, Value};

use crate::ir::{
    bound::{BoundExpr, BoundStatement},
    ids::ExprId,
};

pub struct ExprFormatter<'a> {
    stmt: &'a BoundStatement,
    dialect: Dialect,
}

impl<'a> ExprFormatter<'a> {
    pub fn new(stmt: &'a BoundStatement, dialect: Dialect) -> Self {
        Self { stmt, dialect }
    }

    /// Format an expression as SQL string.
    /// This matches the behavior of different SQL dialects when no alias is provided.
    pub fn format_expr(&self, expr_id: ExprId) -> String {
        let expr = self.stmt.exprs.get(expr_id);
        match expr {
            BoundExpr::Column(column_id) => self.format_column(*column_id),
            BoundExpr::Literal(value) => self.format_literal_as_column_name(value),
            BoundExpr::Function {
                name,
                args,
                distinct,
                ..
            } => self.format_function_as_column_name(name, args, *distinct),
            BoundExpr::Case { .. } => self.format_case_as_column_name(expr_id),
            BoundExpr::Binary { left, op, right } => {
                self.format_binary_as_column_name(*left, op, *right)
            },
            BoundExpr::Unary { op, expr } => self.format_unary_as_column_name(op, *expr),
            BoundExpr::IsNull { expr, negated } => {
                self.format_is_null_as_column_name(*expr, *negated)
            },
            BoundExpr::Wildcard => "*".to_string(),
            _ => self.format_unknown_as_column_name(),
        }
    }

    fn format_column(&self, column_id: crate::ir::ids::ColumnId) -> String {
        self.stmt.columns.get(column_id).name.clone()
    }

    fn format_literal_as_column_name(&self, value: &Value) -> String {
        match self.dialect {
            Dialect::Postgres => "?column?".to_string(),
            _ => self.format_literal(value),
        }
    }

    fn format_function_as_column_name(
        &self,
        name: &str,
        args: &[ExprId],
        distinct: bool,
    ) -> String {
        match self.dialect {
            Dialect::Postgres => name.to_lowercase(),
            _ => self.format_function_call(name, args, distinct),
        }
    }

    fn format_case_as_column_name(&self, expr_id: ExprId) -> String {
        match self.dialect {
            Dialect::Postgres => "case".to_string(),
            _ => self.format_case_expr(expr_id),
        }
    }

    fn format_binary_as_column_name(
        &self,
        left: ExprId,
        op: &BinaryOperator,
        right: ExprId,
    ) -> String {
        match self.dialect {
            Dialect::Postgres => "?column?".to_string(),
            _ => {
                let left_str = self.format_expr(left);
                let right_str = self.format_expr(right);
                let op_str = self.format_binary_op(op);
                format!("{} {} {}", left_str, op_str, right_str)
            },
        }
    }

    fn format_unary_as_column_name(&self, op: &UnaryOperator, expr: ExprId) -> String {
        match self.dialect {
            Dialect::Postgres => "?column?".to_string(),
            _ => {
                let expr_str = self.format_expr(expr);
                let op_str = self.format_unary_op(op);
                format!("{}{}", op_str, expr_str)
            },
        }
    }

    fn format_is_null_as_column_name(&self, expr: ExprId, negated: bool) -> String {
        match self.dialect {
            Dialect::Postgres => "?column?".to_string(),
            _ => {
                let expr_str = self.format_expr(expr);
                if negated {
                    format!("{} IS NOT NULL", expr_str)
                } else {
                    format!("{} IS NULL", expr_str)
                }
            },
        }
    }

    fn format_unknown_as_column_name(&self) -> String {
        match self.dialect {
            Dialect::Postgres => "?column?".to_string(),
            _ => "?".to_string(),
        }
    }

    fn format_function_call(&self, name: &str, args: &[ExprId], distinct: bool) -> String {
        let args_str = args
            .iter()
            .map(|arg| self.format_expr(*arg))
            .collect::<Vec<_>>()
            .join(", ");
        if distinct {
            format!("{}(DISTINCT {})", name, args_str)
        } else {
            format!("{}({})", name, args_str)
        }
    }

    fn format_case_expr(&self, expr_id: ExprId) -> String {
        let expr = self.stmt.exprs.get(expr_id);
        if let BoundExpr::Case {
            operand,
            conditions,
            results,
            else_result,
        } = expr
        {
            let mut parts = vec!["CASE".to_string()];
            if let Some(operand_id) = operand {
                parts.push(self.format_expr(*operand_id));
            }
            for (cond, result) in conditions.iter().zip(results.iter()) {
                parts.push(format!("WHEN {}", self.format_expr(*cond)));
                parts.push(format!("THEN {}", self.format_expr(*result)));
            }
            if let Some(else_id) = else_result {
                parts.push(format!("ELSE {}", self.format_expr(*else_id)));
            }
            parts.push("END".to_string());
            parts.join(" ")
        } else {
            "?".to_string()
        }
    }

    fn format_literal(&self, value: &Value) -> String {
        match value {
            Value::Null => "NULL".to_string(),
            Value::Boolean(b) => {
                if *b {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            },
            Value::Number(num, _) => num.clone(),
            Value::SingleQuotedString(s) | Value::DoubleQuotedString(s) => {
                // Always keep quotes in the formatter
                // The caller (infer_expr_name) will remove them for top-level literals if needed
                format!("'{}'", s)
            },
            _ => "?".to_string(),
        }
    }

    fn format_binary_op(&self, op: &BinaryOperator) -> &'static str {
        match op {
            BinaryOperator::Plus => "+",
            BinaryOperator::Minus => "-",
            BinaryOperator::Multiply => "*",
            BinaryOperator::Divide => "/",
            BinaryOperator::Modulo => "%",
            BinaryOperator::Eq => "=",
            BinaryOperator::NotEq => "!=",
            BinaryOperator::Lt => "<",
            BinaryOperator::LtEq => "<=",
            BinaryOperator::Gt => ">",
            BinaryOperator::GtEq => ">=",
            BinaryOperator::And => "AND",
            BinaryOperator::Or => "OR",
            _ => "?",
        }
    }

    fn format_unary_op(&self, op: &UnaryOperator) -> &'static str {
        match op {
            UnaryOperator::Plus => "+",
            UnaryOperator::Minus => "-",
            UnaryOperator::Not => "NOT ",
            _ => "?",
        }
    }
}
