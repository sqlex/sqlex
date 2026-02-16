use crate::functions::{
    FunctionRegistry,
    model::{FunctionCategory, FunctionNullabilityRule, FunctionReturnTypeRule, FunctionSignature},
};

pub(crate) fn register_window_functions(registry: &mut FunctionRegistry) {
    for name in ["row_number", "rank", "dense_rank"] {
        registry.register(
            name,
            FunctionSignature::new(
                FunctionCategory::Window,
                0,
                Some(0),
                FunctionReturnTypeRule::Ranking,
                FunctionNullabilityRule::Never,
            ),
        );
    }
    for name in ["lead", "lag"] {
        registry.register(
            name,
            FunctionSignature::new(
                FunctionCategory::Window,
                1,
                Some(3),
                FunctionReturnTypeRule::LeadLag,
                FunctionNullabilityRule::Always,
            ),
        );
    }
}
