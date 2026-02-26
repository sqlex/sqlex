use std::collections::HashMap;

use sqlex_analyzer::error::AnalyzerError;
use sqlex_common::dialect::Dialect;

use crate::{
    arena::{Arena, RelationId},
    builder::RelationTree,
    catalog::Catalog,
    node::expression::column_ref::SlotId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputColumn {
    pub(crate) id: SlotId,
    pub(crate) name: String,
    pub(crate) qualifier: Option<String>,
}

impl OutputColumn {
    #[allow(dead_code)]
    pub(crate) fn new(id: SlotId, name: impl Into<String>, qualifier: Option<String>) -> Self {
        Self {
            id,
            name: name.into(),
            qualifier,
        }
    }
}

#[derive(Debug)]
pub(crate) struct BoundRelationTree {
    #[allow(dead_code)]
    pub(crate) arena: Arena,
    #[allow(dead_code)]
    pub(crate) root: RelationId,
    #[allow(dead_code)]
    pub(crate) outputs: HashMap<RelationId, Vec<OutputColumn>>,
}

impl BoundRelationTree {
    #[allow(dead_code)]
    pub(crate) fn new(
        arena: Arena,
        root: RelationId,
        outputs: HashMap<RelationId, Vec<OutputColumn>>,
    ) -> Result<Self, AnalyzerError> {
        arena.resolve(root)?;
        Ok(Self {
            arena,
            root,
            outputs,
        })
    }
}

#[derive(Debug)]
pub(crate) struct RelationBinder<'a> {
    #[allow(dead_code)]
    pub(crate) dialect: Dialect,
    #[allow(dead_code)]
    pub(crate) catalog: &'a Catalog,
    pub(crate) arena: Arena,
    pub(crate) root: RelationId,
    #[allow(dead_code)]
    pub(crate) outputs: HashMap<RelationId, Vec<OutputColumn>>,
    #[allow(dead_code)]
    pub(crate) next_slot_id: u32,
}

impl<'a> RelationBinder<'a> {
    pub(crate) fn new(dialect: Dialect, catalog: &'a Catalog, tree: RelationTree) -> Self {
        let (arena, root) = tree.into_parts();
        Self {
            dialect,
            catalog,
            arena,
            root,
            outputs: HashMap::new(),
            next_slot_id: 1,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn allocate_slot_id(&mut self) -> Result<SlotId, AnalyzerError> {
        let next_slot_id = self.next_slot_id;
        self.next_slot_id = self
            .next_slot_id
            .checked_add(1)
            .ok_or_else(|| AnalyzerError::analysis("A0010", "slot id space exhausted"))?;
        Ok(SlotId(next_slot_id))
    }

    pub(crate) fn bind(self) -> Result<BoundRelationTree, AnalyzerError> {
        let _ = self.arena.resolve(self.root)?;
        let _ = (
            &self.dialect,
            self.catalog,
            &self.outputs,
            self.next_slot_id,
        );
        todo!("relation binder is not implemented yet")
    }
}
