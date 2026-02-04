use sqlex_analyzer::Result;
use sqlex_common::DataType;

use super::super::order_by::OrderByExpr;
use crate::planner::{
    BuildContext,
    expr::{Expression, ExpressionNode},
    scope::Scope,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateFunctionName {
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

impl AggregateFunctionName {
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
}

/// Aggregate function expression
#[derive(Debug, Clone)]
pub struct AggregateFunctionExpr {
    pub function: AggregateFunctionName,
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
    pub fn build(
        ctx: &mut BuildContext,
        func: &sqlparser::ast::Function,
        scope: &Scope,
    ) -> Result<Self> {
        use sqlex_analyzer::ObjectNameExt;
        use sqlparser::ast::{
            DuplicateTreatment, FunctionArg, FunctionArgExpr, FunctionArguments, Value,
        };

        use super::super::values::LiteralExpr;

        let name = func.name.to_dotted_string().to_uppercase();
        let function =
            AggregateFunctionName::from_name(&name).unwrap_or(AggregateFunctionName::Custom(name));

        let mut args = Vec::new();
        let mut distinct = false;

        if let FunctionArguments::List(ref list) = func.args {
            for arg in &list.args {
                match arg {
                    FunctionArg::Named {
                        arg: FunctionArgExpr::Expr(e),
                        ..
                    } => {
                        args.push(ctx.build_expr(e, scope)?);
                    },
                    FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) => {
                        args.push(ctx.build_expr(e, scope)?);
                    },
                    FunctionArg::Unnamed(FunctionArgExpr::Wildcard) => {
                        // COUNT(*) -> 1
                        args.push(Box::new(LiteralExpr::new(Value::Number(
                            "1".to_string(),
                            false,
                        ))));
                    },
                    _ => {},
                }
            }
            distinct = list.duplicate_treatment == Some(DuplicateTreatment::Distinct);
        }

        let filter = if let Some(filter) = &func.filter {
            Some(ctx.build_expr(filter, scope)?)
        } else {
            None
        };

        let order_by = Vec::new();

        let arg_types: Vec<DataType> = args.iter().map(|e| e.data_type()).collect();
        let arg_nullables: Vec<bool> = args.iter().map(|e| e.nullable()).collect();

        // Infer return type and nullability
        let input_type = arg_types.first().cloned().unwrap_or(DataType::Int);
        let _input_nullable = arg_nullables.first().cloned().unwrap_or(true);

        let (return_type, is_nullable) = match &function {
            AggregateFunctionName::Count => (DataType::BigInt, false),
            AggregateFunctionName::Sum => {
                let ret_type = match input_type {
                    DataType::TinyInt | DataType::SmallInt | DataType::Int | DataType::BigInt => {
                        DataType::BigInt
                    },
                    DataType::Float | DataType::Double | DataType::Decimal => DataType::Double,
                    _ => input_type,
                };
                (ret_type, true) // SUM always returns NULL on empty set
            },
            AggregateFunctionName::Avg => (DataType::Double, true),
            AggregateFunctionName::Min | AggregateFunctionName::Max => (input_type, true),
            AggregateFunctionName::First | AggregateFunctionName::Last => (input_type, true),
            AggregateFunctionName::ArrayAgg => (DataType::Array(Box::new(input_type)), true),
            AggregateFunctionName::JsonArrayAgg | AggregateFunctionName::JsonObjectAgg => {
                (DataType::Json, true)
            },
            AggregateFunctionName::StringAgg => (DataType::Text, true),
            AggregateFunctionName::Custom(_) => (input_type, true),
        };

        Ok(Self {
            function,
            args,
            distinct,
            filter,
            order_by,
            return_type,
            is_nullable,
        })
    }
}
