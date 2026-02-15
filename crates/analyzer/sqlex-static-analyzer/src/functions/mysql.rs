use crate::functions::registry::{
    FunctionCategory, FunctionNullabilityRule, FunctionRegistry, FunctionReturnTypeRule,
    FunctionSignature,
};

pub(crate) fn register_mysql_functions(registry: &mut FunctionRegistry) {
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
