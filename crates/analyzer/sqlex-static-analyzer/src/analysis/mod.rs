use sqlex_common::dialect::Dialect;

use crate::{analysis::diagnostics::Diagnostic, catalog::Catalog, ir::output::OutputSchema};

pub mod bind;
pub mod diagnostics;
pub mod functions;
pub mod infer;
pub(crate) mod keywords;
pub mod validate;

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
        // Phase 1: Bind
        let bind_result = bind::Binder::new(self.dialect, catalog).bind(sql);
        let Some(bound) = bind_result.bound else {
            return AnalysisResult {
                output: None,
                diagnostics: bind_result.diagnostics,
            };
        };
        let mut diagnostics = bind_result.diagnostics;

        // Phase 2: Validate
        let validation = validate::Validator::new(self.dialect).validate(&bound);
        diagnostics.extend(validation.diagnostics);

        // Phase 3: Infer
        let infer_result = infer::Inferrer::new(self.dialect, catalog).infer(&bound);
        diagnostics.extend(infer_result.diagnostics);

        AnalysisResult {
            output: infer_result.output,
            diagnostics,
        }
    }
}
