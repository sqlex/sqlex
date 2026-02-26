use crate::functions::{
    FunctionRegistry,
    model::{FunctionCategory, FunctionNullabilityRule, FunctionReturnTypeRule, FunctionSignature},
};

pub(crate) fn register_numeric_functions(registry: &mut FunctionRegistry) {
    for name in ["abs", "ceil", "floor", "sqrt", "exp", "ln", "log10", "sign"] {
        registry.register(
            name,
            FunctionSignature::new(
                FunctionCategory::Scalar,
                1,
                Some(1),
                FunctionReturnTypeRule::NumericUnary,
                FunctionNullabilityRule::AnyArg,
            ),
        );
    }
    registry.register(
        "round",
        FunctionSignature::new(
            FunctionCategory::Scalar,
            1,
            Some(2),
            FunctionReturnTypeRule::NumericUnary,
            FunctionNullabilityRule::AnyArg,
        ),
    );
    registry.register(
        "power",
        FunctionSignature::new(
            FunctionCategory::Scalar,
            2,
            Some(2),
            FunctionReturnTypeRule::NumericBinaryCommon,
            FunctionNullabilityRule::AnyArg,
        ),
    );
    registry.register(
        "mod",
        FunctionSignature::new(
            FunctionCategory::Scalar,
            2,
            Some(2),
            FunctionReturnTypeRule::NumericBinaryCommon,
            FunctionNullabilityRule::AnyArg,
        ),
    );
}
