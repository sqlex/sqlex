use std::collections::HashMap;

use sqlex_common::dialect::Dialect;
use sqlparser::ast::Statement;

use crate::{
    algebraizer::{
        model::relation::Relation,
        scope::{CteScope, QueryScope},
    },
    catalog::Catalog,
    diagnostics::{Diagnostic, Phase},
    functions::FunctionRegistry,
};

pub(crate) mod model;

mod cte;
mod expression;
mod from_join;
mod from_table_factor;
mod join;
mod query;
mod scope;
mod select;
mod set_ops;

#[derive(Debug)]
pub(crate) struct Algebraizer<'a> {
    dialect: Dialect,
    catalog: &'a Catalog,
    functions: &'a FunctionRegistry,
    query_scope_stack: Vec<QueryScope>,
    cte_scope_stack: Vec<CteScope>,
    next_relation_id: u32,
    next_slot_id: u32,
}

impl<'a> Algebraizer<'a> {
    pub(crate) fn new(
        dialect: Dialect,
        catalog: &'a Catalog,
        functions: &'a FunctionRegistry,
    ) -> Self {
        Self {
            dialect,
            catalog,
            functions,
            query_scope_stack: vec![QueryScope {
                relation_bindings: Vec::new(),
                named_windows: HashMap::new(),
                literal_assignment_mode: false,
            }],
            cte_scope_stack: vec![HashMap::new()],
            next_relation_id: 1,
            next_slot_id: 1,
        }
    }

    pub(crate) fn build(mut self, statement: &Statement) -> Result<Relation, Diagnostic> {
        let Statement::Query(query) = statement else {
            return Err(Diagnostic::new(
                "A3001",
                Phase::Algebraize,
                "only query statements are supported in analyze",
            ));
        };

        self.build_query_relation(query, false)
    }
}
