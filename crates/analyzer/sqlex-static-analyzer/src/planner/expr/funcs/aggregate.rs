use sqlex_analyzer::{ObjectNameExt, Result};
use sqlex_common::DataType;

use super::super::{Expression, ExpressionNode};

/// Aggregate function
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    ArrayAgg,
    StringAgg,
    JsonAgg,
    First,
    Last,
    Custom(String),
}

fn promote_to_large(t: DataType) -> DataType {
    use DataType::*;
    match t {
        TinyInt | SmallInt | Int => BigInt,
        Float => Double,
        other => other,
    }
}

impl AggregateFunction {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_uppercase().as_str() {
            "COUNT" => Some(Self::Count),
            "SUM" => Some(Self::Sum),
            "AVG" => Some(Self::Avg),
            "MIN" => Some(Self::Min),
            "MAX" => Some(Self::Max),
            "ARRAY_AGG" => Some(Self::ArrayAgg),
            "STRING_AGG" => Some(Self::StringAgg),
            "JSON_AGG" => Some(Self::JsonAgg),
            "FIRST_VALUE" | "FIRST" => Some(Self::First),
            "LAST_VALUE" | "LAST" => Some(Self::Last),
            _ => None,
        }
    }

    /// Infer type for the new Expression system (takes DataType slices)
    pub fn infer_type(&self, arg_types: &[DataType], _arg_nullables: &[bool]) -> (DataType, bool) {
        let input_type = arg_types.first().cloned().unwrap_or(DataType::Int);

        match self {
            AggregateFunction::Count => (DataType::BigInt, false),
            AggregateFunction::Sum => (promote_to_large(input_type), true),
            AggregateFunction::Avg => (DataType::Double, true),
            AggregateFunction::Min | AggregateFunction::Max => (input_type, true),
            AggregateFunction::ArrayAgg => (DataType::Array(Box::new(input_type)), true),
            AggregateFunction::StringAgg => (DataType::Text, true),
            AggregateFunction::JsonAgg => (DataType::Json, true),
            AggregateFunction::First | AggregateFunction::Last => (input_type, true),
            AggregateFunction::Custom(_) => (input_type, true),
        }
    }
}

/// Aggregate function expression
#[derive(Debug, Clone)]
pub struct AggregateFunctionExpr {
    pub function: AggregateFunction,
    pub args: Vec<Box<dyn Expression>>,
    pub return_type: DataType,
    pub is_nullable: bool,
}

impl ExpressionNode for AggregateFunctionExpr {
    fn data_type(&self) -> DataType {
        self.return_type.clone()
    }

    fn nullable(&self) -> bool {
        self.is_nullable
    }
}

impl AggregateFunctionExpr {
    /// Build an aggregate function expression
    pub fn build(
        function: AggregateFunction,
        args: Vec<Box<dyn Expression>>,
    ) -> Box<dyn Expression> {
        // Infer return type using the aggregate function's logic
        let arg_types: Vec<DataType> = args.iter().map(|e| e.data_type()).collect();
        let arg_nullables: Vec<bool> = args.iter().map(|e| e.nullable()).collect();
        let (return_type, is_nullable) = function.infer_type(&arg_types, &arg_nullables);

        Box::new(AggregateFunctionExpr {
            function,
            args,
            return_type,
            is_nullable,
        })
    }
}

use super::super::order_by::OrderByExpr;

/// Legacy Aggregate expression (updated to use new Expression system)
#[derive(Debug, Clone)]
pub struct AggregateExpr {
    pub function: AggregateFunction,
    pub args: Vec<Box<dyn Expression>>,
    pub distinct: bool,
    pub filter: Option<Box<dyn Expression>>,
    pub order_by: Vec<OrderByExpr>,
}

impl AggregateExpr {
    pub fn build(
        function: AggregateFunction,
        args: Vec<Box<dyn Expression>>,
        distinct: bool,
        filter: Option<Box<dyn Expression>>,
        order_by: Vec<OrderByExpr>,
    ) -> Self {
        Self {
            function,
            args,
            distinct,
            filter,
            order_by,
        }
    }

    pub fn from_ast<F>(func: &sqlparser::ast::Function, mut expr_builder: F) -> Result<Self>
    where
        F: FnMut(&sqlparser::ast::Expr) -> Result<Box<dyn Expression>>,
    {
        use sqlparser::ast::{
            DuplicateTreatment, FunctionArg, FunctionArgExpr, FunctionArguments, Value,
        };

        use super::super::values::LiteralExpr;

        let name = func.name.to_dotted_string().to_uppercase();
        let function =
            AggregateFunction::from_name(&name).unwrap_or(AggregateFunction::Custom(name));

        let mut args = Vec::new();
        let mut distinct = false;

        if let FunctionArguments::List(ref list) = func.args {
            for arg in &list.args {
                match arg {
                    FunctionArg::Named {
                        arg: FunctionArgExpr::Expr(e),
                        ..
                    } => {
                        args.push(expr_builder(e)?);
                    },
                    FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) => {
                        args.push(expr_builder(e)?);
                    },
                    FunctionArg::Unnamed(FunctionArgExpr::Wildcard) => {
                        // COUNT(*) -> 1
                        args.push(LiteralExpr::build(Value::Number("1".to_string(), false)));
                    },
                    _ => {},
                }
            }
            distinct = list.duplicate_treatment == Some(DuplicateTreatment::Distinct);
        }

        if args.is_empty() && matches!(function, AggregateFunction::Count) {
            args.push(LiteralExpr::build(Value::Number("1".to_string(), false)));
        }

        let filter = if let Some(filter) = &func.filter {
            Some(expr_builder(filter)?)
        } else {
            None
        };

        Ok(Self::build(
            function,
            args,
            distinct,
            filter,
            Vec::new(), // TODO: Order By support
        ))
    }
}
