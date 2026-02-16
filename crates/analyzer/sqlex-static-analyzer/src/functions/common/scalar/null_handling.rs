use crate::functions::{
    FunctionRegistry,
    model::{FunctionCategory, FunctionNullabilityRule, FunctionReturnTypeRule, FunctionSignature},
};

pub(crate) fn register_null_handling_functions(registry: &mut FunctionRegistry) {
    registry.register(
        "coalesce",
        FunctionSignature::new(
            FunctionCategory::Scalar,
            1,
            None,
            FunctionReturnTypeRule::CoalesceCommonType,
            FunctionNullabilityRule::AllArgs,
        ),
    );
    registry.register(
        "nullif",
        FunctionSignature::new(
            FunctionCategory::Scalar,
            2,
            Some(2),
            FunctionReturnTypeRule::NullIfFirstArg,
            FunctionNullabilityRule::Always,
        ),
    );
}
