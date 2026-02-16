use sqlex_common::dialect::Dialect;
use sqlparser::ast::Statement;

use crate::{
    algebraizer::{
        context::BuildContext,
        model::{relation::Relation, schema::OutputSchema},
    },
    catalog::Catalog,
    diagnostics::{Diagnostic, Phase},
    functions::FunctionRegistry,
};

pub(crate) mod model;

mod context;
mod cte;
mod expression;
mod from_join;
mod from_table_factor;
mod join;
mod query;
mod select;
mod set_ops;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Algebraizer {
    dialect: Dialect,
}

impl Algebraizer {
    pub(crate) fn new(dialect: Dialect) -> Self {
        Self { dialect }
    }

    pub(crate) fn build(
        &self,
        statement: &Statement,
        catalog: &Catalog,
        functions: &FunctionRegistry,
    ) -> Result<Relation, Diagnostic> {
        let Statement::Query(query) = statement else {
            return Err(Diagnostic::new(
                "A3001",
                Phase::Algebraize,
                "only query statements are supported in analyze",
            ));
        };

        let mut context = BuildContext::new();
        self.build_query_relation(query, catalog, functions, &mut context, false)
    }
}

fn output_schema_of(relation: &Relation) -> Result<OutputSchema, Diagnostic> {
    match relation {
        Relation::Scan(node) => Ok(node.schema.clone()),
        Relation::Values(node) => Ok(node.schema.clone()),
        Relation::Selection(node) => Ok(node.schema.clone()),
        Relation::Projection(node) => Ok(node.schema.clone()),
        Relation::Aggregation(node) => Ok(node.schema.clone()),
        Relation::Window(node) => Ok(node.schema.clone()),
        Relation::Distinct(node) => Ok(node.schema.clone()),
        Relation::Sort(node) => Ok(node.schema.clone()),
        Relation::Limit(node) => Ok(node.schema.clone()),
        Relation::Alias(node) => Ok(node.schema.clone()),
        Relation::Join(node) => Ok(node.schema.clone()),
        Relation::SetOperation(node) => Ok(node.schema.clone()),
    }
}
