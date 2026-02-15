use crate::functions::{
    FunctionRegistry,
    model::{FunctionArgTypeRule, FunctionCategory, FunctionCoercionProfile},
};

pub(crate) fn register_postgres_functions(registry: &mut FunctionRegistry) {
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
    for name in ["sum", "avg"] {
        registry.set_arg_type_rules(
            name,
            FunctionCategory::Aggregate,
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
