use crate::functions::{
    FunctionRegistry,
    model::{FunctionCategory, FunctionNullabilityRule, FunctionReturnTypeRule, FunctionSignature},
};

pub(crate) fn register_null_handling_functions(registry: &mut FunctionRegistry) {
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
}
