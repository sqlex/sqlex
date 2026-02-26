use sqlex_analyzer::error::AnalyzerError;
use sqlex_common::dialect::Dialect;
use sqlparser::ast;

use crate::{
    arena::{Arena, RelationId},
    catalog::Catalog,
    functions::FunctionRegistry,
};

mod expression;
mod relation;

#[derive(Debug)]
pub(crate) struct RelationBuilder<'a> {
    dialect: Dialect,
    catalog: &'a Catalog,
    functions: &'a FunctionRegistry,
    arena: Arena,
}

impl<'a> RelationBuilder<'a> {
    pub(crate) fn new(
        dialect: Dialect,
        catalog: &'a Catalog,
        functions: &'a FunctionRegistry,
    ) -> Self {
        Self {
            dialect,
            catalog,
            functions,
            arena: Arena::new(),
        }
    }

    pub(crate) fn build(
        mut self,
        statement: ast::Statement,
    ) -> Result<RelationTree, AnalyzerError> {
        let root = match statement {
            ast::Statement::Query(query) => self.build_query(*query)?,
            _ => {
                return Err(AnalyzerError::todo(
                    "statement builder is not implemented yet",
                ));
            },
        };

        RelationTree::new(self.arena, root)
    }
}

#[derive(Debug)]
pub(crate) struct RelationTree {
    #[allow(dead_code)]
    arena: Arena,
    #[allow(dead_code)]
    root: RelationId,
}

impl RelationTree {
    pub(crate) fn new(arena: Arena, root_id: RelationId) -> Result<Self, AnalyzerError> {
        arena.resolve(root_id)?;

        Ok(Self {
            arena,
            root: root_id,
        })
    }

    pub(crate) fn into_parts(self) -> (Arena, RelationId) {
        (self.arena, self.root)
    }
}
