use crate::functions::FunctionRegistry;

mod null_handling;
mod numeric;

pub(crate) fn register_scalar_functions(registry: &mut FunctionRegistry) {
    null_handling::register_null_handling_functions(registry);
    numeric::register_numeric_type_rules(registry);
}
