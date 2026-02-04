use super::{TypeContext, TypeInfo};
use crate::analysis::functions::{
    AggregateFunctionName, FunctionKind, WindowFunctionName, resolve_function,
};

impl<'a> TypeContext<'a> {
    pub(super) fn infer_function(
        &mut self,
        name: &str,
        arg_types: &[sqlex_common::DataType],
        arg_nullables: &[bool],
        distinct: bool,
        over: bool,
    ) -> TypeInfo {
        let upper = name.to_uppercase();
        let meta = resolve_function(&upper);

        if meta.requires_over && !over {
            self.diagnostics
                .push(super::super::diagnostics::Diagnostic::window_requires_over(
                    &upper,
                ));
        }
        if over && !meta.allows_over {
            self.diagnostics
                .push(super::super::diagnostics::Diagnostic::over_not_allowed(
                    &upper,
                ));
        }
        if distinct && !meta.accepts_distinct {
            self.diagnostics
                .push(super::super::diagnostics::Diagnostic::distinct_not_allowed(
                    &upper,
                ));
        }
        if !meta.arity.matches(arg_types.len()) {
            self.diagnostics.push(
                super::super::diagnostics::Diagnostic::function_arity_mismatch(
                    &upper,
                    &meta.arity.describe(),
                    arg_types.len(),
                ),
            );
        }

        match meta.kind {
            FunctionKind::Window(window) => self.infer_window_function(window, arg_types),
            FunctionKind::Aggregate(agg) => {
                if over {
                    self.infer_window_function(WindowFunctionName::Aggregate(agg), arg_types)
                } else {
                    self.infer_aggregate_function(agg, arg_types)
                }
            },
            FunctionKind::Scalar(scalar) => {
                let (data_type, nullable) = scalar.infer_type(arg_types, arg_nullables);
                TypeInfo {
                    data_type,
                    nullable,
                }
            },
            FunctionKind::Unknown => {
                self.diagnostics
                    .push(super::super::diagnostics::Diagnostic::unknown_function(
                        &upper,
                    ));
                TypeInfo {
                    data_type: sqlex_common::DataType::Custom("unknown".to_string()),
                    nullable: true,
                }
            },
        }
    }

    fn infer_aggregate_function(
        &self,
        function: AggregateFunctionName,
        arg_types: &[sqlex_common::DataType],
    ) -> TypeInfo {
        use sqlex_common::DataType;

        let input_type = arg_types.first().cloned().unwrap_or(DataType::Int);
        match function {
            AggregateFunctionName::Count => TypeInfo {
                data_type: DataType::BigInt,
                nullable: false,
            },
            AggregateFunctionName::Sum => TypeInfo {
                data_type: match input_type {
                    DataType::TinyInt | DataType::SmallInt | DataType::Int | DataType::BigInt => {
                        DataType::BigInt
                    },
                    DataType::Float | DataType::Double | DataType::Decimal => DataType::Double,
                    _ => input_type,
                },
                nullable: true,
            },
            AggregateFunctionName::Avg => TypeInfo {
                data_type: DataType::Double,
                nullable: true,
            },
            AggregateFunctionName::Min | AggregateFunctionName::Max => TypeInfo {
                data_type: input_type,
                nullable: true,
            },
            AggregateFunctionName::First | AggregateFunctionName::Last => TypeInfo {
                data_type: input_type,
                nullable: true,
            },
            AggregateFunctionName::ArrayAgg => TypeInfo {
                data_type: DataType::Array(Box::new(input_type)),
                nullable: true,
            },
            AggregateFunctionName::JsonArrayAgg | AggregateFunctionName::JsonObjectAgg => {
                TypeInfo {
                    data_type: DataType::Json,
                    nullable: true,
                }
            },
            AggregateFunctionName::StringAgg => TypeInfo {
                data_type: DataType::Text,
                nullable: true,
            },
            AggregateFunctionName::Custom(_) => TypeInfo {
                data_type: input_type,
                nullable: true,
            },
        }
    }

    fn infer_window_function(
        &self,
        function: WindowFunctionName,
        arg_types: &[sqlex_common::DataType],
    ) -> TypeInfo {
        use sqlex_common::DataType;

        match function {
            WindowFunctionName::RowNumber
            | WindowFunctionName::Rank
            | WindowFunctionName::DenseRank
            | WindowFunctionName::NTile => TypeInfo {
                data_type: DataType::BigInt,
                nullable: false,
            },
            WindowFunctionName::PercentRank | WindowFunctionName::CumeDist => TypeInfo {
                data_type: DataType::Double,
                nullable: false,
            },
            WindowFunctionName::Lead
            | WindowFunctionName::Lag
            | WindowFunctionName::FirstValue
            | WindowFunctionName::LastValue
            | WindowFunctionName::NthValue => TypeInfo {
                data_type: arg_types.first().cloned().unwrap_or(DataType::Int),
                nullable: true,
            },
            WindowFunctionName::Aggregate(agg) => self.infer_aggregate_function(agg, arg_types),
        }
    }
}
