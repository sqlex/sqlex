use sqlex_common::dialect::Dialect;

use crate::{
    algebraizer::model::relation::Relation,
    catalog::Catalog,
    diagnostics::Diagnostic,
    functions::FunctionRegistry,
    infer::{metadata::InferMetadata, operator_infer::infer_operator},
};

pub(crate) mod cardinality;
pub(crate) mod metadata;
pub(crate) mod operator_infer;
pub(crate) mod scalar_infer;

#[derive(Debug, Clone)]
pub(crate) struct Inferencer {
    dialect: Dialect,
    functions: FunctionRegistry,
}

impl Inferencer {
    pub(crate) fn new(dialect: Dialect, functions: &FunctionRegistry) -> Self {
        Self {
            dialect,
            functions: functions.clone(),
        }
    }

    pub(crate) fn infer(
        &self,
        expr: &Relation,
        catalog: &Catalog,
    ) -> Result<InferMetadata, Diagnostic> {
        infer_operator(expr, catalog, self.dialect, &self.functions)
    }
}
