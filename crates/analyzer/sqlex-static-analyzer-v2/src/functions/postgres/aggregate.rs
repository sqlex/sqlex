use crate::functions::{
    FunctionRegistry,
    model::{FunctionArgTypeRule, FunctionCategory, FunctionCoercionProfile},
};

pub(crate) fn register_aggregate_functions(registry: &mut FunctionRegistry) {
    let numeric_arg_0 = [FunctionArgTypeRule::numeric(0)];
    for name in ["sum", "avg"] {
        registry.set_arg_type_rules(
            name,
            FunctionCategory::Aggregate,
            FunctionCoercionProfile::Strict,
            &numeric_arg_0,
        );
    }
}
