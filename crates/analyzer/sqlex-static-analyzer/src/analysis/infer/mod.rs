use std::collections::{HashMap, HashSet};

use sqlex_common::dialect::Dialect;

use crate::{
    analysis::diagnostics::Diagnostic,
    catalog::Catalog,
    ir::{bound::BoundQuery, output::OutputSchema},
};

mod cardinality;
mod functions;
mod grouping;
mod inference;
mod lineage;
mod schema;

#[derive(Debug, Clone)]
pub(super) struct TypeInfo {
    data_type: sqlex_common::types::DataType,
    nullable: bool,
}

pub struct InferResult {
    pub output: Option<OutputSchema>,
    pub diagnostics: Vec<Diagnostic>,
}

pub(super) struct Inferrer<'a> {
    #[allow(dead_code)]
    dialect: Dialect,
    catalog: &'a Catalog,
    diagnostics: Vec<Diagnostic>,
    schema_cache: HashMap<usize, OutputSchema>,
}

impl<'a> Inferrer<'a> {
    pub(super) fn new(dialect: Dialect, catalog: &'a Catalog) -> Self {
        Self {
            dialect,
            catalog,
            diagnostics: Vec::new(),
            schema_cache: HashMap::new(),
        }
    }

    pub(super) fn infer(mut self, bound: &BoundQuery) -> InferResult {
        let output = Some(self.output_schema_for_query(bound));
        InferResult {
            output,
            diagnostics: self.diagnostics,
        }
    }
}

pub(super) struct QueryTypeState<'a> {
    query: &'a BoundQuery,
    types: HashMap<crate::ir::ids::ExprId, TypeInfo>,
    nullable_tables: HashSet<crate::ir::ids::TableId>,
}

impl<'a> QueryTypeState<'a> {
    fn new(query: &'a BoundQuery) -> Self {
        Self {
            query,
            types: HashMap::new(),
            nullable_tables: HashSet::new(),
        }
    }
}
