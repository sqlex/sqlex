use sqlex_analyzer::Result;
use sqlex_common::DataType;

use super::super::order_by::OrderByExpr;
use crate::planner::expr::{Expression, ExpressionNode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    First,
    Last,
    ArrayAgg,
    JsonArrayAgg,
    JsonObjectAgg,
    StringAgg,
    Custom(String),
}

impl AggregateFunction {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_uppercase().as_str() {
            "COUNT" => Some(Self::Count),
            "SUM" => Some(Self::Sum),
            "AVG" => Some(Self::Avg),
            "MIN" => Some(Self::Min),
            "MAX" => Some(Self::Max),
            "FIRST" => Some(Self::First),
            "LAST" => Some(Self::Last),
            "ARRAY_AGG" => Some(Self::ArrayAgg),
            "JSON_AGG" | "JSON_ARRAYAGG" => Some(Self::JsonArrayAgg),
            "JSON_OBJECT_AGG" | "JSON_OBJECTAGG" => Some(Self::JsonObjectAgg),
            "STRING_AGG" | "GROUP_CONCAT" | "LISTAGG" => Some(Self::StringAgg),
            _ => None,
        }
    }

    /// Infer result type and nullability
    pub fn infer_type(&self, arg_types: &[DataType], arg_nullables: &[bool]) -> (DataType, bool) {
        let input_type = arg_types.first().cloned().unwrap_or(DataType::Int);
        // Default nullability: if input is nullable, output is nullable.
        // Except COUNT which is never null.
        // Default nullability: if input is nullable, output is nullable.
        // Except COUNT which is never null.
        let _input_nullable = arg_nullables.first().cloned().unwrap_or(true);

        match self {
            AggregateFunction::Count => (DataType::BigInt, false),
            AggregateFunction::Sum => {
                let ret_type = match input_type {
                    DataType::TinyInt | DataType::SmallInt | DataType::Int | DataType::BigInt => {
                        DataType::BigInt
                    },
                    DataType::Float | DataType::Double | DataType::Decimal => DataType::Double,
                    _ => input_type,
                };
                (ret_type, true) // SUM always returns NULL on empty set
            },
            AggregateFunction::Avg => (DataType::Double, true),
            AggregateFunction::Min | AggregateFunction::Max => (input_type, true),
            AggregateFunction::First | AggregateFunction::Last => (input_type, true),
            AggregateFunction::ArrayAgg => (DataType::Array(Box::new(input_type)), true),
            AggregateFunction::JsonArrayAgg | AggregateFunction::JsonObjectAgg => {
                (DataType::Json, true)
            },
            AggregateFunction::StringAgg => (DataType::Text, true),
            AggregateFunction::Custom(_) => (input_type, true),
        }
    }
}

/// Aggregate function expression
#[derive(Debug, Clone)]
pub struct AggregateFunctionExpr {
    pub function: AggregateFunction,
    pub args: Vec<Box<dyn Expression>>,
    pub distinct: bool,
    pub filter: Option<Box<dyn Expression>>,
    pub order_by: Vec<OrderByExpr>,
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
        distinct: bool,
        filter: Option<Box<dyn Expression>>,
        order_by: Vec<OrderByExpr>,
    ) -> Self {
        let arg_types: Vec<DataType> = args.iter().map(|e| e.data_type()).collect();
        let arg_nullables: Vec<bool> = args.iter().map(|e| e.nullable()).collect();
        let (return_type, is_nullable) = function.infer_type(&arg_types, &arg_nullables);

        Self {
            function,
            args,
            distinct,
            filter,
            order_by,
            return_type,
            is_nullable,
        }
    }

    pub fn from_ast<F>(func: &sqlparser::ast::Function, mut expr_builder: F) -> Result<Self>
    where
        F: FnMut(&sqlparser::ast::Expr) -> Result<Box<dyn Expression>>,
    {
        use sqlex_analyzer::ObjectNameExt;
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

        let filter = if let Some(filter) = &func.filter {
            Some(expr_builder(filter)?)
        } else {
            None
        };

        let order_by = Vec::new();

        Ok(Self::build(function, args, distinct, filter, order_by))
    }
}
