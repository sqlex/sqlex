use crate::analysis::{
    diagnostics::Diagnostic,
    functions::{FunctionKind, WindowFunction, resolve_function},
    infer::{Inferrer, TypeInfo},
};

impl Inferrer {
    pub(super) fn infer_function(
        &mut self,
        name: &str,
        kind: &FunctionKind,
        arg_types: &[sqlex_common::types::DataType],
        arg_nullables: &[bool],
        _distinct: bool,
        over: bool,
    ) -> TypeInfo {
        let upper = name.to_uppercase();
        let meta = resolve_function(&upper);

        // Type-related validation (requires knowing types)
        if meta.arity.matches(arg_types.len()) {
            if let Some(detail) = meta.validate_argument_types(self.dialect, arg_types) {
                self.diagnostics
                    .push(Diagnostic::function_argument_type_mismatch(&upper, detail));
            }
        }

        match kind {
            FunctionKind::Window(window) => {
                let (data_type, nullable) = window.infer_type(self.dialect, arg_types);
                TypeInfo {
                    data_type,
                    nullable,
                }
            },
            FunctionKind::Aggregate(agg) => {
                let (data_type, nullable) = if over {
                    WindowFunction::Aggregate(agg.clone()).infer_type(self.dialect, arg_types)
                } else {
                    agg.infer_type(self.dialect, arg_types)
                };
                TypeInfo {
                    data_type,
                    nullable,
                }
            },
            FunctionKind::Scalar(scalar) => {
                let (data_type, nullable) =
                    scalar.infer_type(self.dialect, arg_types, arg_nullables);
                TypeInfo {
                    data_type,
                    nullable,
                }
            },
            FunctionKind::Unknown => TypeInfo {
                data_type: sqlex_common::types::DataType::Custom("unknown".to_string()),
                nullable: true,
            },
        }
    }
}
