use crate::functions::FunctionRegistry;

mod scalar;

pub(crate) fn register_sqlite_functions(registry: &mut FunctionRegistry) {
    scalar::register_scalar_functions(registry);
}
