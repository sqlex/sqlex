use std::collections::{HashMap, HashSet};

use sqlex_common::dialect::Dialect;

use crate::{
    analysis::diagnostics::Diagnostic,
    catalog::Catalog,
    ir::{bound::BoundStatement, ids::ExprId, output::OutputSchema},
};

mod cardinality;
mod functions;
mod inference;
mod schema;

#[derive(Debug, Clone)]
pub(crate) struct TypeInfo {
    data_type: sqlex_common::types::DataType,
    nullable: bool,
}

pub struct InferResult {
    pub output: Option<OutputSchema>,
    pub diagnostics: Vec<Diagnostic>,
}

pub(crate) struct Inferrer<'a> {
    pub(super) dialect: Dialect,
    pub(super) catalog: &'a Catalog,
    pub(super) diagnostics: Vec<Diagnostic>,
    schema_cache: HashMap<SchemaCacheKey, OutputSchema>,
}

/// Cache key using arena indices to avoid raw pointer issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
enum SchemaCacheKey {
    TopLevel,
    Cte(usize),
    Derived(crate::ir::ids::TableId),
    Subquery(ExprId),
}

impl<'a> Inferrer<'a> {
    pub(crate) fn new(dialect: Dialect, catalog: &'a Catalog) -> Self {
        Self {
            dialect,
            catalog,
            diagnostics: Vec::new(),
            schema_cache: HashMap::new(),
        }
    }

    pub(crate) fn infer(mut self, stmt: &BoundStatement) -> InferResult {
        let output = Some(self.output_schema_for_statement(stmt));
        InferResult {
            output,
            diagnostics: self.diagnostics,
        }
    }
}

pub(super) struct QueryTypeState<'a> {
    pub(super) stmt: &'a BoundStatement,
    pub(super) types: HashMap<ExprId, TypeInfo>,
    pub(super) nullable_tables: HashSet<crate::ir::ids::TableId>,
}

impl<'a> QueryTypeState<'a> {
    fn new(stmt: &'a BoundStatement) -> Self {
        Self {
            stmt,
            types: HashMap::new(),
            nullable_tables: HashSet::new(),
        }
    }
}
