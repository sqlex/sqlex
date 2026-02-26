use crate::functions::{
    FunctionRegistry,
    model::{FunctionCategory, FunctionNullabilityRule, FunctionReturnTypeRule, FunctionSignature},
};

pub(crate) fn register_text_functions(registry: &mut FunctionRegistry) {
    for name in [
        "upper",
        "lower",
        "trim",
        "ltrim",
        "rtrim",
        "length",
        "char_length",
    ] {
        let return_type_rule = if matches!(name, "length" | "char_length") {
            FunctionReturnTypeRule::LengthInteger
        } else {
            FunctionReturnTypeRule::TextLikeOrDefaultText
        };
        registry.register(
            name,
            FunctionSignature::new(
                FunctionCategory::Scalar,
                1,
                Some(1),
                return_type_rule,
                FunctionNullabilityRule::AnyArg,
            ),
        );
    }
    registry.register(
        "substr",
        FunctionSignature::new(
            FunctionCategory::Scalar,
            2,
            Some(3),
            FunctionReturnTypeRule::TextLikeOrDefaultText,
            FunctionNullabilityRule::AnyArg,
        ),
    );
}
