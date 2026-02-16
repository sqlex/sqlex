use sqlex_common::dialect::Dialect;

use crate::{
    algebraizer::model::relation::Relation, catalog::Catalog, diagnostics::Diagnostic,
    functions::FunctionRegistry, infer::metadata::InferMetadata,
};

pub(crate) mod cardinality;
pub(crate) mod metadata;
pub(crate) mod operator_infer;
pub(crate) mod scalar_infer;

#[derive(Debug, Clone)]
pub(crate) struct Inferencer<'a> {
    dialect: Dialect,
    catalog: &'a Catalog,
    functions: &'a FunctionRegistry,
}

impl<'a> Inferencer<'a> {
    pub(crate) fn new(
        dialect: Dialect,
        catalog: &'a Catalog,
        functions: &'a FunctionRegistry,
    ) -> Self {
        Self {
            dialect,
            catalog,
            functions,
        }
    }

    pub(crate) fn infer(&self, relation: &Relation) -> Result<InferMetadata, Diagnostic> {
        self.infer_operator(relation)
    }
}
