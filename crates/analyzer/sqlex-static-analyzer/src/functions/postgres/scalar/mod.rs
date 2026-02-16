use crate::functions::FunctionRegistry;

mod numeric;
mod text;

pub(crate) fn register_scalar_functions(registry: &mut FunctionRegistry) {
    text::register_text_type_rules(registry);
    numeric::register_numeric_type_rules(registry);
}
