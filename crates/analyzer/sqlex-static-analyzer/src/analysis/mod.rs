pub mod bind;
pub mod diagnostics;
pub(crate) mod functions;
pub mod typecheck;

use diagnostics::Diagnostic;

use crate::{catalog::Catalog, ir::OutputSchema};

pub struct AnalysisResult {
    pub output: Option<OutputSchema>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn analyze(catalog: &Catalog, sql: &str) -> AnalysisResult {
    let bind_result = bind::bind(catalog, sql);

    if let Some(bound) = bind_result.bound {
        let mut diagnostics = bind_result.diagnostics;
        let type_result = typecheck::typecheck(catalog, &bound);
        diagnostics.extend(type_result.diagnostics);
        AnalysisResult {
            output: type_result.output,
            diagnostics,
        }
    } else {
        AnalysisResult {
            output: None,
            diagnostics: bind_result.diagnostics,
        }
    }
}
