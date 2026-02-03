use sqlex_common::DataType;

use crate::planner::expr::{Expression, ExpressionNode};

/// Column reference expression
#[derive(Debug, Clone)]
pub struct ColumnExpr {
    /// Optional table qualifier
    pub table: Option<String>,
    /// Column name
    pub column: String,
    /// Resolved data type
    pub return_type: DataType,
    /// Whether the column is nullable
    pub is_nullable: bool,
}

impl ExpressionNode for ColumnExpr {
    fn data_type(&self) -> DataType {
        self.return_type.clone()
    }

    fn nullable(&self) -> bool {
        self.is_nullable
    }
}

impl ColumnExpr {
    /// Build a column expression with resolved type information
    pub fn build(
        table: Option<String>,
        column: String,
        data_type: DataType,
        nullable: bool,
    ) -> Box<dyn Expression> {
        Box::new(ColumnExpr {
            table,
            column,
            return_type: data_type,
            is_nullable: nullable,
        })
    }
}
