mod functions;
mod grouping;
mod infer;
mod lineage;
mod schema;

use std::collections::{HashMap, HashSet};

use super::diagnostics::Diagnostic;
use crate::{
    catalog::Catalog,
    ir::{BoundQuery, OutputSchema},
};

#[derive(Debug, Clone)]
pub(super) struct TypeInfo {
    data_type: sqlex_common::DataType,
    nullable: bool,
}

pub struct TypecheckResult {
    pub output: Option<OutputSchema>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn typecheck(catalog: &Catalog, bound: &BoundQuery) -> TypecheckResult {
    let mut ctx = TypeContext::new(catalog);
    let output = Some(ctx.output_schema_for_query(bound));
    TypecheckResult {
        output,
        diagnostics: ctx.diagnostics,
    }
}

pub(super) struct TypeContext<'a> {
    catalog: &'a Catalog,
    diagnostics: Vec<Diagnostic>,
    schema_cache: HashMap<usize, OutputSchema>,
}

impl<'a> TypeContext<'a> {
    fn new(catalog: &'a Catalog) -> Self {
        Self {
            catalog,
            diagnostics: Vec::new(),
            schema_cache: HashMap::new(),
        }
    }
}

pub(super) struct QueryTypeState<'a> {
    query: &'a BoundQuery,
    types: HashMap<crate::ir::ExprId, TypeInfo>,
    nullable_tables: HashSet<crate::ir::TableId>,
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
