use crate::functions::registry::{FunctionCategory, FunctionRegistry};

pub(crate) fn register_common_functions(registry: &mut FunctionRegistry) {
    for name in [
        "upper",
        "lower",
        "trim",
        "ltrim",
        "rtrim",
        "length",
        "char_length",
    ] {
        registry.register(name, FunctionCategory::Scalar, 1, Some(1));
    }
    registry.register("substr", FunctionCategory::Scalar, 2, Some(3));

    registry.register("coalesce", FunctionCategory::Scalar, 1, None);
    registry.register("nullif", FunctionCategory::Scalar, 2, Some(2));
    for name in ["abs", "ceil", "floor", "sqrt", "exp", "ln", "log10", "sign"] {
        registry.register(name, FunctionCategory::Scalar, 1, Some(1));
    }
    registry.register("round", FunctionCategory::Scalar, 1, Some(2));
    registry.register("power", FunctionCategory::Scalar, 2, Some(2));
    registry.register("mod", FunctionCategory::Scalar, 2, Some(2));

    for name in ["count", "sum", "avg", "min", "max"] {
        registry.register(name, FunctionCategory::Aggregate, 1, Some(1));
    }

    for name in ["row_number", "rank", "dense_rank", "lead", "lag"] {
        registry.register(name, FunctionCategory::Window, 0, None);
    }
}
