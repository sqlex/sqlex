use crate::functions::{
    FunctionRegistry,
    model::{FunctionArgTypeRule, FunctionCategory, FunctionCoercionProfile},
};

pub(crate) fn register_numeric_type_rules(registry: &mut FunctionRegistry) {
    let numeric_arg_0 = [FunctionArgTypeRule::numeric(0)];
    for name in ["ceil", "floor"] {
        registry.set_arg_type_rules(
            name,
            FunctionCategory::Scalar,
            FunctionCoercionProfile::Strict,
            &numeric_arg_0,
        );
    }
}
