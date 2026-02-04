use super::{TypeContext, TypeInfo};
use crate::analysis::functions::{FunctionKind, WindowFunction, resolve_function};

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
            FunctionKind::Window(window) => {
                let (data_type, nullable) = window.infer_type(arg_types);
                TypeInfo {
                    data_type,
                    nullable,
                }
            },
            FunctionKind::Aggregate(agg) => {
                let (data_type, nullable) = if over {
                    WindowFunction::Aggregate(agg).infer_type(arg_types)
                } else {
                    agg.infer_type(arg_types)
                };
                TypeInfo {
                    data_type,
                    nullable,
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
}
