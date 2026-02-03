use sqlex_analyzer::AnalyzerError;
use sqlparser::ast::{SetOperator, SetQuantifier};

use crate::planner::plan::{LogicalNode, PlanNode, PlanNodeColumn};

type Result<T> = std::result::Result<T, AnalyzerError>;

/// Set operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOp {
    Union,
    Intersect,
    Except,
}

#[derive(Debug, Clone)]
pub struct SetOperationNode {
    pub op: SetOp,
    pub all: bool,
    pub left: Box<dyn PlanNode>,
    pub right: Box<dyn PlanNode>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl SetOperationNode {
    /// Build from AST SetOperator and SetQuantifier
    pub fn from_ast(
        op: &SetOperator,
        set_quantifier: &SetQuantifier,
        left: Box<dyn PlanNode>,
        right: Box<dyn PlanNode>,
    ) -> Result<Self> {
        // Convert SetOperator to SetOp
        let set_op = match op {
            SetOperator::Union => SetOp::Union,
            SetOperator::Intersect => SetOp::Intersect,
            SetOperator::Except => SetOp::Except,
            _ => {
                return Err(AnalyzerError::AnalysisError(
                    "Unsupported set operator".to_string(),
                ));
            },
        };

        // Convert SetQuantifier to all flag
        let all = matches!(
            set_quantifier,
            SetQuantifier::All | SetQuantifier::AllByName
        );

        // Use left side's columns (SQL standard: names come from left)
        let output_columns = left.columns().to_vec();

        Ok(Self {
            op: set_op,
            all,
            left,
            right,
            output_columns,
        })
    }
}

impl LogicalNode for SetOperationNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
