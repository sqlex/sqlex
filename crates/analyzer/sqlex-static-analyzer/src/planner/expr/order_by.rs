use super::Expression;

/// ORDER BY expression
#[derive(Debug, Clone)]
pub struct OrderByExpr {
    pub expr: Box<dyn Expression>,
    pub asc: bool,
    pub nulls_first: Option<bool>,
}

impl OrderByExpr {
    /// Build OrderByExpr (logic-only, no AST parsing)
    pub fn build(expr: Box<dyn Expression>, asc: bool, nulls_first: Option<bool>) -> Self {
        Self {
            expr,
            asc,
            nulls_first,
        }
    }
}
