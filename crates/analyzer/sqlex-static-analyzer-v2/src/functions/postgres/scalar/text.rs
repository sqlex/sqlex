use crate::functions::{
    FunctionRegistry,
    model::{FunctionArgTypeRule, FunctionCategory, FunctionCoercionProfile},
};

pub(crate) fn register_text_type_rules(registry: &mut FunctionRegistry) {
    let text_arg_0 = [FunctionArgTypeRule::text(0)];
    for name in [
        "upper",
        "lower",
        "trim",
        "ltrim",
        "rtrim",
        "length",
        "char_length",
        "substr",
    ] {
        registry.set_arg_type_rules(
            name,
            FunctionCategory::Scalar,
            FunctionCoercionProfile::Strict,
            &text_arg_0,
        );
    }
}
