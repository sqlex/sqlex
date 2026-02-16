use crate::functions::{
    FunctionRegistry,
    model::{FunctionCategory, FunctionNullabilityRule, FunctionReturnTypeRule, FunctionSignature},
};

pub(crate) fn register_aggregate_functions(registry: &mut FunctionRegistry) {
    registry.register(
        "count",
        FunctionSignature::new(
            FunctionCategory::Aggregate,
            1,
            Some(1),
            FunctionReturnTypeRule::Count,
            FunctionNullabilityRule::Never,
        ),
    );
    registry.register(
        "sum",
        FunctionSignature::new(
            FunctionCategory::Aggregate,
            1,
            Some(1),
            FunctionReturnTypeRule::Sum,
            FunctionNullabilityRule::Always,
        ),
    );
    registry.register(
        "avg",
        FunctionSignature::new(
            FunctionCategory::Aggregate,
            1,
            Some(1),
            FunctionReturnTypeRule::Avg,
            FunctionNullabilityRule::Always,
        ),
    );
    for name in ["min", "max"] {
        registry.register(
            name,
            FunctionSignature::new(
                FunctionCategory::Aggregate,
                1,
                Some(1),
                FunctionReturnTypeRule::MinMax,
                FunctionNullabilityRule::Always,
            ),
        );
    }
}
