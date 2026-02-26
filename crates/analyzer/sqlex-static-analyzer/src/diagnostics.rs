use std::fmt;

use sqlex_analyzer::error::AnalyzerError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Parse,
    Algebraize,
    Infer,
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Parse => "PARSE",
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

    pub(crate) fn into_execution_error(self) -> AnalyzerError {
        AnalyzerError::analysis(self.code, format!("[{}] {}", self.phase, self.message))
    }

    pub(crate) fn into_analysis_error(self) -> AnalyzerError {
        AnalyzerError::analysis(self.code, format!("[{}] {}", self.phase, self.message))
    }
}
