use sqlex_analyzer::{AnalyzerError, Result};
use sqlex_common::DataType;

use crate::planner::{
    expr::{Expression, ExpressionNode},
    scope::Scope,
};

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
    pub(crate) fn new(
        table: Option<String>,
        column: String,
        data_type: DataType,
        nullable: bool,
    ) -> Self {
        ColumnExpr {
            table,
            column,
            return_type: data_type,
            is_nullable: nullable,
        }
    }

    pub fn build(expr: &sqlparser::ast::Expr, scope: &Scope) -> Result<Box<dyn Expression>> {
        match expr {
            sqlparser::ast::Expr::Identifier(ident) => {
                let col = scope.resolve_column(None, &ident.value)?;
                Ok(Box::new(Self::new(
                    None,
                    ident.value.clone(),
                    col.data_type,
                    col.nullable,
                )))
            },
            sqlparser::ast::Expr::CompoundIdentifier(idents) => {
                if idents.len() == 2 {
                    let col = scope.resolve_column(Some(&idents[0].value), &idents[1].value)?;
                    Ok(Box::new(Self::new(
                        Some(idents[0].value.clone()),
                        idents[1].value.clone(),
                        col.data_type,
                        col.nullable,
                    )))
                } else {
                    Err(AnalyzerError::AnalysisError(
                        "Deep compound identifiers not supported".to_string(),
                    ))
                }
            },
            _ => Err(AnalyzerError::AnalysisError(
                "Invalid expression for ColumnExpr".to_string(),
            )),
        }
    }
}
