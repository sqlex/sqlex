use crate::functions::{
    FunctionRegistry,
    model::{FunctionArgTypeRule, FunctionCategory, FunctionCoercionProfile},
};

pub(crate) fn register_numeric_type_rules(registry: &mut FunctionRegistry) {
    let numeric_arg_0 = [FunctionArgTypeRule::numeric(0)];
    for name in [
        "abs", "ceil", "floor", "round", "sqrt", "exp", "ln", "log10", "sign",
    ] {
        registry.set_arg_type_rules(
            name,
            FunctionCategory::Scalar,
            FunctionCoercionProfile::Strict,
            &numeric_arg_0,
        );
    }

    let numeric_args_0_1 = [
        FunctionArgTypeRule::numeric(0),
        FunctionArgTypeRule::numeric(1),
    ];
    for name in ["power", "mod"] {
        registry.set_arg_type_rules(
            name,
            FunctionCategory::Scalar,
            FunctionCoercionProfile::Strict,
            &numeric_args_0_1,
        );
    }
}
