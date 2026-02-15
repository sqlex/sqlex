use std::fmt;

use sqlex_analyzer::AnalyzerError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Parse,
    Catalog,
    Algebraize,
    Infer,
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Parse => "PARSE",
            Self::Catalog => "CATALOG",
            Self::Algebraize => "ALGEBRAIZE",
            Self::Infer => "INFER",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Diagnostic {
    pub(crate) code: &'static str,
    pub(crate) phase: Phase,
    pub(crate) message: String,
}

impl Diagnostic {
    pub(crate) fn new(code: &'static str, phase: Phase, message: impl Into<String>) -> Self {
        Self {
            code,
            phase,
            message: message.into(),
        }
    }

    pub(crate) fn render(&self) -> String {
        format!("[{}:{}] {}", self.phase, self.code, self.message)
    }

    pub(crate) fn into_execution_error(self) -> AnalyzerError {
        AnalyzerError::ExecutionError(self.render())
    }

    pub(crate) fn into_analysis_error(self) -> AnalyzerError {
        AnalyzerError::AnalysisError(self.render())
    }
}
