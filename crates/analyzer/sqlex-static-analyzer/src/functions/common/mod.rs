use crate::functions::FunctionRegistry;

mod aggregate;
mod scalar;
mod window;

pub(crate) fn register_common_functions(registry: &mut FunctionRegistry) {
    scalar::register_scalar_functions(registry);
    aggregate::register_aggregate_functions(registry);
    window::register_window_functions(registry);
}
