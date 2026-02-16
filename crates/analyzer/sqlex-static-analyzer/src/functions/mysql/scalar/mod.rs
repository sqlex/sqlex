use crate::functions::FunctionRegistry;

mod null_handling;

pub(crate) fn register_scalar_functions(registry: &mut FunctionRegistry) {
    null_handling::register_null_handling_functions(registry);
}
