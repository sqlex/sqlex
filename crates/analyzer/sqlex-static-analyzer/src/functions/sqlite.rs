use crate::functions::{
    FunctionRegistry,
    model::{
        FunctionArgTypeRule, FunctionCategory, FunctionCoercionProfile, FunctionNullabilityRule,
        FunctionReturnTypeRule, FunctionSignature,
    },
};

pub(crate) fn register_sqlite_functions(registry: &mut FunctionRegistry) {
    registry.register(
        "ifnull",
        FunctionSignature::new(
            FunctionCategory::Scalar,
            2,
            Some(2),
            FunctionReturnTypeRule::CoalesceCommonType,
            FunctionNullabilityRule::AllArgs,
        ),
    );

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
