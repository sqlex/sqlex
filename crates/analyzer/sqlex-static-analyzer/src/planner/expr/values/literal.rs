use sqlex_common::DataType;
use sqlparser::ast::Value;

use crate::planner::expr::{Expression, ExpressionNode};

/// Literal value expression
#[derive(Debug, Clone)]
pub struct LiteralExpr {
    pub value: Value,
    pub return_type: DataType,
}

impl ExpressionNode for LiteralExpr {
    fn data_type(&self) -> DataType {
        self.return_type.clone()
    }

    fn nullable(&self) -> bool {
        matches!(self.value, Value::Null)
    }
}

impl LiteralExpr {
    /// Build a literal expression from an AST Value
    pub(crate) fn new(value: Value) -> Self {
        let return_type = infer_literal_type(&value);
        LiteralExpr { value, return_type }
    }

    pub fn build(value: &Value) -> Box<dyn Expression> {
        Box::new(Self::new(value.clone()))
    }
}

/// Infer the data type of a literal value
fn infer_literal_type(v: &Value) -> DataType {
    match v {
        Value::Number(num, _) => {
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
