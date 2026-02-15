use sqlex_common::dialect::Dialect;

use crate::{
    algebra::expr::RelExpr,
    catalog::model::Catalog,
    diagnostics::Diagnostic,
    functions::registry::FunctionRegistry,
    infer::{metadata::InferMetadata, operator_infer::infer_operator},
};

#[derive(Debug, Clone)]
pub(crate) struct InferEngine {
    dialect: Dialect,
    functions: FunctionRegistry,
}

impl InferEngine {
    pub(crate) fn new(dialect: Dialect, functions: &FunctionRegistry) -> Self {
        Self {
            dialect,
            functions: functions.clone(),
        }
    }

    pub(crate) fn infer(
        &self,
        expr: &RelExpr,
        catalog: &Catalog,
    ) -> Result<InferMetadata, Diagnostic> {
        infer_operator(expr, catalog, self.dialect, &self.functions)
    }
}
