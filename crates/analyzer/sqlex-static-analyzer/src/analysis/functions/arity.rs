#[derive(Debug, Clone, Copy)]
pub(crate) enum FunctionArity {
    Any,
    Exact(usize),
    AtLeast(usize),
    Between { min: usize, max: usize },
}

impl FunctionArity {
    pub(crate) fn matches(self, count: usize) -> bool {
        match self {
            Self::Any => true,
            Self::Exact(expected) => count == expected,
            Self::AtLeast(min) => count >= min,
            Self::Between { min, max } => count >= min && count <= max,
        }
    }

    pub(crate) fn describe(self) -> String {
        match self {
            Self::Any => "any number of".to_string(),
            Self::Exact(expected) => format!("{expected}"),
            Self::AtLeast(min) => format!("at least {min}"),
            Self::Between { min, max } => format!("between {min} and {max}"),
        }
    }
}
