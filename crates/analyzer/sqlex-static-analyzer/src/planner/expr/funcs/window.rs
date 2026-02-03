use sqlex_analyzer::{AnalyzerError, ObjectNameExt, Result};
use sqlex_common::DataType;

use super::{
    super::{Expression, ExpressionNode, order_by::OrderByExpr},
    aggregate::AggregateFunction,
};

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

/// Window function name
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowFunctionName {
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

impl WindowFunctionName {
    pub fn from_name(name: &str) -> Option<Self> {
        let name_upper = name.to_uppercase();
        match name_upper.as_str() {
            "ROW_NUMBER" => Some(Self::RowNumber),
            "RANK" => Some(Self::Rank),
            "DENSE_RANK" => Some(Self::DenseRank),
            "NTILE" => Some(Self::NTile),
            "LEAD" => Some(Self::Lead),
            "LAG" => Some(Self::Lag),
            "FIRST_VALUE" => Some(Self::FirstValue),
            "LAST_VALUE" => Some(Self::LastValue),
            "NTH_VALUE" => Some(Self::NthValue),
            "PERCENT_RANK" => Some(Self::PercentRank),
            "CUME_DIST" => Some(Self::CumeDist),
            _ => None,
        }
    }
}

/// Window function expression
#[derive(Debug, Clone)]
pub struct WindowFunctionExpr {
    pub function: WindowFunctionName,
    pub args: Vec<Box<dyn Expression>>,
    pub partition_by: Vec<Box<dyn Expression>>,
    pub order_by: Vec<OrderByExpr>,
    pub frame: Option<WindowFrame>,
    pub return_type: DataType,
    pub is_nullable: bool,
}

impl ExpressionNode for WindowFunctionExpr {
    fn data_type(&self) -> DataType {
        self.return_type.clone()
    }

    fn nullable(&self) -> bool {
        self.is_nullable
    }
}

impl WindowFunctionExpr {
    /// Build a window function expression
    pub fn build(
        function: WindowFunctionName,
        args: Vec<Box<dyn Expression>>,
        partition_by: Vec<Box<dyn Expression>>,
        order_by: Vec<OrderByExpr>,
        frame: Option<WindowFrame>,
    ) -> Box<dyn Expression> {
        // Infer return type using local logic (logic moved from WindowFunction::infer_type)
        let arg_types: Vec<DataType> = args.iter().map(|e| e.data_type()).collect();
        let (return_type, is_nullable) = match &function {
            WindowFunctionName::Aggregate(agg) => agg.infer_type(&arg_types, &[]),
            WindowFunctionName::RowNumber
            | WindowFunctionName::Rank
            | WindowFunctionName::DenseRank
            | WindowFunctionName::NTile => (DataType::BigInt, false),
            WindowFunctionName::Lead
            | WindowFunctionName::Lag
            | WindowFunctionName::FirstValue
            | WindowFunctionName::LastValue
            | WindowFunctionName::NthValue => {
                let input_type = arg_types.first().cloned().unwrap_or(DataType::Int);
                (input_type, true)
            },
            WindowFunctionName::PercentRank | WindowFunctionName::CumeDist => {
                (DataType::Double, false)
            },
        };

        Box::new(WindowFunctionExpr {
            function,
            args,
            partition_by,
            order_by,
            frame,
            return_type,
            is_nullable,
        })
    }

    pub fn from_ast<F>(
        func: &sqlparser::ast::Function,
        args: Vec<Box<dyn Expression>>,
        mut expr_builder: F,
    ) -> Result<Box<dyn Expression>>
    where
        F: FnMut(&sqlparser::ast::Expr) -> Result<Box<dyn Expression>>,
    {
        use sqlparser::ast::{WindowFrameUnits as SQLWindowFrameUnits, WindowType};

        let name = func.name.to_dotted_string();
        let over = func.over.as_ref().ok_or_else(|| {
            AnalyzerError::AnalysisError("Window function requires OVER clause".to_string())
        })?;

        let window_func = if let Some(wf) = WindowFunctionName::from_name(&name) {
            wf
        } else if let Some(af) = AggregateFunction::from_name(&name) {
            WindowFunctionName::Aggregate(af)
        } else {
            return Err(AnalyzerError::AnalysisError(format!(
                "Unknown window function: {}",
                name
            )));
        };

        let (partition_by_exprs, order_by_exprs, window_frame) = match over {
            WindowType::WindowSpec(spec) => {
                let mut partition_by = Vec::new();
                for expr in &spec.partition_by {
                    partition_by.push(expr_builder(expr)?);
                }

                let mut order_by = Vec::new();
                for ob in &spec.order_by {
                    let expr = expr_builder(&ob.expr)?;
                    order_by.push(OrderByExpr::build(
                        expr,
                        ob.asc.unwrap_or(true),
                        ob.nulls_first,
                    ));
                }

                let frame = if let Some(frame) = &spec.window_frame {
                    let units = match frame.units {
                        SQLWindowFrameUnits::Rows => WindowFrameUnits::Rows,
                        SQLWindowFrameUnits::Range => WindowFrameUnits::Range,
                        SQLWindowFrameUnits::Groups => WindowFrameUnits::Groups,
                    };

                    let convert_bound = |b: &sqlparser::ast::WindowFrameBound| {
                        match b {
                            sqlparser::ast::WindowFrameBound::CurrentRow => {
                                WindowFrameBound::CurrentRow
                            },
                            sqlparser::ast::WindowFrameBound::Preceding(_) => {
                                // Simplifying frame bound handling for now
                                WindowFrameBound::Preceding(None)
                            },
                            sqlparser::ast::WindowFrameBound::Following(_) => {
                                WindowFrameBound::Following(None)
                            },
                        }
                    };

                    Some(WindowFrame {
                        units,
                        start: convert_bound(&frame.start_bound),
                        end: frame.end_bound.as_ref().map(convert_bound),
                    })
                } else {
                    None
                };

                (partition_by, order_by, frame)
            },
            WindowType::NamedWindow(_) => {
                return Err(AnalyzerError::AnalysisError(
                    "Named windows not yet supported".to_string(),
                ));
            },
        };

        Ok(Self::build(
            window_func,
            args,
            partition_by_exprs,
            order_by_exprs,
            window_frame,
        ))
    }
}
