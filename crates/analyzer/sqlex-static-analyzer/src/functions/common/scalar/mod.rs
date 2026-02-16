use crate::functions::FunctionRegistry;

mod null_handling;
mod numeric;
mod text;

pub(crate) fn register_scalar_functions(registry: &mut FunctionRegistry) {
    text::register_text_functions(registry);
    null_handling::register_null_handling_functions(registry);
    numeric::register_numeric_functions(registry);
}
