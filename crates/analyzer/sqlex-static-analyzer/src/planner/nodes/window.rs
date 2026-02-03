use sqlex_common::DataType;

use crate::planner::{
    expr::{AggregateFunction, OrderByExpr, TypedExpr, aggregate_result_type},
    plan::{LogicalNode, PlanNode, PlanNodeColumn},
};

/// Window expression
#[derive(Debug, Clone)]
pub struct WindowExpr {
    pub function: WindowFunction,
    pub args: Vec<TypedExpr>,
    pub partition_by: Vec<TypedExpr>,
    pub order_by: Vec<OrderByExpr>,
    pub frame: Option<WindowFrame>,
}

/// Window frame specification
#[derive(Debug, Clone)]
pub struct WindowFrame {
    pub units: WindowFrameUnits,
    pub start: WindowFrameBound,
    pub end: Option<WindowFrameBound>,
}

/// Window frame units
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowFrameUnits {
    Rows,
    Range,
    Groups,
}

/// Window frame bound
#[derive(Debug, Clone)]
pub enum WindowFrameBound {
    CurrentRow,
    Preceding(Option<u64>),
    Following(Option<u64>),
}

/// Window function
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowFunction {
    /// Aggregate function used as window function
    Aggregate(AggregateFunction),
    /// Dedicated window functions
    RowNumber,
    Rank,
    DenseRank,
    NTile,
    Lead,
    Lag,
    FirstValue,
    LastValue,
    NthValue,
    PercentRank,
    CumeDist,
}

/// Determine the result type and nullability of a window function.
pub fn window_result_type(func: &WindowFunction, args: &[TypedExpr]) -> (DataType, bool) {
    match func {
        WindowFunction::Aggregate(agg) => aggregate_result_type(agg, args),
        WindowFunction::RowNumber
        | WindowFunction::Rank
        | WindowFunction::DenseRank
        | WindowFunction::NTile => (DataType::BigInt, false),
        WindowFunction::Lead
        | WindowFunction::Lag
        | WindowFunction::FirstValue
        | WindowFunction::LastValue
        | WindowFunction::NthValue => {
            let input_type = args
                .first()
                .map(|a| a.data_type.clone())
                .unwrap_or(DataType::Int);
            (input_type, true) // These can return NULL
        },
        WindowFunction::PercentRank | WindowFunction::CumeDist => (DataType::Double, false),
    }
}

#[derive(Debug, Clone)]
pub struct WindowNode {
    pub input: Box<dyn PlanNode>,
    pub functions: Vec<WindowExpr>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl WindowNode {
    pub fn build(input: Box<dyn PlanNode>, functions: Vec<WindowExpr>) -> Self {
        let mut output_columns = input.columns().to_vec();
        for (i, win) in functions.iter().enumerate() {
            let (data_type, _nullable) = window_result_type(&win.function, &win.args);
            output_columns.push(PlanNodeColumn {
                name: format!("window_{}", i),
                data_type,
                nullability: true,
                origin_table: None,
                origin_column: None,
            });
        }
        Self {
            input,
            functions,
            output_columns,
        }
    }
}

impl LogicalNode for WindowNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
