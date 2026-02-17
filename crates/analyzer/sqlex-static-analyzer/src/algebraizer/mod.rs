use sqlex_common::dialect::Dialect;
use sqlparser::ast::Statement;

use crate::{
    algebraizer::{
        model::relation::Relation,
        scope::{
            CteScopeStack, LiteralAssignmentModeStack, NamedWindowScopeStack, RelationScopeStack,
        },
    },
    catalog::Catalog,
    diagnostics::{Diagnostic, Phase},
    functions::FunctionRegistry,
};

pub(crate) mod model;

mod expression;
mod relation;
mod scope;

#[derive(Debug)]
pub(crate) struct Algebraizer<'a> {
    dialect: Dialect,
    catalog: &'a Catalog,
    functions: &'a FunctionRegistry,
    pub(crate) relation_scope: RelationScopeStack,
    pub(crate) cte_scope: CteScopeStack,
    pub(crate) named_window_scope: NamedWindowScopeStack,
    pub(crate) literal_scope: LiteralAssignmentModeStack,
    pub(crate) next_relation_id: u32,
    pub(crate) next_slot_id: u32,
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
            relation_scope: RelationScopeStack::new(),
            cte_scope: CteScopeStack::new(),
            named_window_scope: NamedWindowScopeStack::new(),
            literal_scope: LiteralAssignmentModeStack::new(),
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

        self.build_query_relation(query)
    }

    pub(crate) fn allocate_relation_id(&mut self) -> u32 {
        let relation_id = self.next_relation_id;
        self.next_relation_id += 1;
        relation_id
    }

    pub(crate) fn allocate_slot_id(&mut self) -> u32 {
        let slot_id = self.next_slot_id;
        self.next_slot_id += 1;
        slot_id
    }
}
