use crate::analysis::{
    diagnostics::Diagnostic,
    functions::Function,
    infer::{Inferrer, TypeInfo},
};

impl Inferrer<'_> {
    pub(super) fn infer_function(
        &mut self,
        name: &str,
        function: &Function,
        arg_types: &[sqlex_common::types::DataType],
        arg_nullables: &[bool],
        _distinct: bool,
        over: bool,
    ) -> TypeInfo {
        let upper = name.to_uppercase();

        // Type-related validation (requires knowing types)
        if function.arity().matches(arg_types.len()) {
            if let Some(detail) = function.validate_argument_types(self.dialect, arg_types) {
                self.diagnostics
                    .push(Diagnostic::function_argument_type_mismatch(&upper, detail));
            }
        }

        let (data_type, nullable) =
            function.infer_type(self.dialect, arg_types, arg_nullables, over);
        TypeInfo {
            data_type,
            nullable,
        }
    }
}
