use sqlex_common::DataType;

use crate::planner::expr::{Expression, ExpressionNode};

/// CAST expression
#[derive(Debug, Clone)]
pub struct CastExpr {
    /// The expression being cast
    pub expr: Box<dyn Expression>,
    /// Target data type
    pub target_type: DataType,
}

impl ExpressionNode for CastExpr {
    fn data_type(&self) -> DataType {
        self.target_type.clone()
    }

    fn nullable(&self) -> bool {
        // Casting preserves nullability of the source expression
        self.expr.nullable()
    }
}

impl CastExpr {
    /// Build a CAST expression
    pub fn build(expr: Box<dyn Expression>, target_type: DataType) -> Box<dyn Expression> {
        Box::new(CastExpr { expr, target_type })
    }
}
