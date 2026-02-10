use sqlex_common::{dialect::Dialect, types::DataType};

mod aggregate;
mod arity;
mod scalar;
mod window;

pub use aggregate::AggregateFunction;
pub(crate) use arity::FunctionArity;
pub use scalar::ScalarFunction;
pub use window::WindowFunction;

#[derive(Debug, Clone)]
pub enum Function {
    Scalar(ScalarFunction),
    Aggregate(AggregateFunction),
    Window(WindowFunction),
    Unknown,
}

impl Function {
    pub(crate) fn accepts_distinct(&self) -> bool {
        matches!(self, Self::Aggregate(_))
    }

    pub(crate) fn allows_over(&self) -> bool {
        matches!(self, Self::Aggregate(_) | Self::Window(_))
    }

    pub(crate) fn requires_over(&self) -> bool {
        matches!(self, Self::Window(_))
    }

    pub(crate) fn arity(&self) -> FunctionArity {
        match self {
            Self::Scalar(f) => f.arity(),
            Self::Aggregate(f) => f.arity(),
            Self::Window(f) => f.arity(),
            Self::Unknown => FunctionArity::Any,
        }
    }

    pub(crate) fn validate_argument_types(
        &self,
        dialect: Dialect,
        arg_types: &[DataType],
    ) -> Option<String> {
        match self {
            Self::Scalar(f) => f.validate_argument_types(dialect, arg_types),
            Self::Aggregate(f) => f.validate_argument_types(dialect, arg_types),
            Self::Window(f) => f.validate_argument_types(dialect, arg_types),
            Self::Unknown => None,
        }
    }

    pub(crate) fn infer_type(
        &self,
        dialect: Dialect,
        arg_types: &[DataType],
        arg_nullables: &[bool],
        over: bool,
    ) -> (DataType, bool) {
        match self {
            Self::Scalar(f) => f.infer_type(dialect, arg_types, arg_nullables),
            Self::Aggregate(f) => {
                if over {
                    WindowFunction::Aggregate(f.clone()).infer_type(dialect, arg_types)
                } else {
                    f.infer_type(dialect, arg_types)
                }
            },
            Self::Window(f) => f.infer_type(dialect, arg_types),
            Self::Unknown => (DataType::Custom("unknown".to_string()), true),
        }
    }
}

pub(crate) fn resolve_function(name: &str) -> Function {
    let upper = name.to_uppercase();

    if let Some(window) = WindowFunction::from_name(&upper) {
        return Function::Window(window);
    }

    if let Some(agg) = AggregateFunction::from_name(&upper) {
        return Function::Aggregate(agg);
    }

    if let Some(scalar) = ScalarFunction::from_name(&upper) {
        return Function::Scalar(scalar);
    }

    Function::Unknown
}
