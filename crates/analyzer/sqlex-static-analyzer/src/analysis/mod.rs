use sqlex_common::dialect::Dialect;

use crate::{analysis::diagnostics::Diagnostic, catalog::Catalog, ir::output::OutputSchema};

pub mod bind;
pub mod diagnostics;
pub(crate) mod functions;
pub(crate) mod keywords;
pub mod typecheck;

pub struct AnalysisResult {
    pub output: Option<OutputSchema>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct AnalysisEngine {
    dialect: Dialect,
}

impl AnalysisEngine {
    pub fn new(dialect: Dialect) -> Self {
        Self { dialect }
    }

    pub fn analyze(&self, catalog: &Catalog, sql: &str) -> AnalysisResult {
        let bind_result = bind::Binder::new(self.dialect, catalog).bind(sql);

        if let Some(bound) = bind_result.bound {
            let mut diagnostics = bind_result.diagnostics;
            let type_result = typecheck::TypeContext::new(self.dialect, catalog).typecheck(&bound);
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
}
