use crate::functions::FunctionRegistry;

mod aggregate;
mod scalar;

pub(crate) fn register_postgres_functions(registry: &mut FunctionRegistry) {
    scalar::register_scalar_functions(registry);
    aggregate::register_aggregate_functions(registry);
}
