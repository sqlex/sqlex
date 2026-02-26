use std::{
    collections::HashMap,
    convert::TryFrom,
    sync::atomic::{AtomicUsize, Ordering},
};

use sqlex_analyzer::error::AnalyzerError;

use crate::node::relation::Relation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RelationId(usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArenaError {
    RelationNotFound(RelationId),
    IdExhausted,
}

impl From<ArenaError> for AnalyzerError {
    fn from(value: ArenaError) -> Self {
        match value {
            ArenaError::RelationNotFound(relation_id) => AnalyzerError::analysis(
                "A0000",
                format!("relation '{relation_id:?}' was not found in arena"),
            ),
            ArenaError::IdExhausted => {
                AnalyzerError::analysis("A0003", "arena relation id space exhausted")
            },
        }
    }
}

#[derive(Debug)]
pub struct Arena {
    relations: HashMap<RelationId, Relation>,
    #[allow(dead_code)]
    next_id: AtomicUsize,
}

impl Arena {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            relations: HashMap::new(),
            next_id: AtomicUsize::new(1),
        }
    }

    pub(crate) fn insert(
        &mut self,
        relation: impl Into<Relation>,
    ) -> Result<RelationId, ArenaError> {
        let relation_id = self
            .next_id
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                current.checked_add(1)
            })
            .map(RelationId)
            .map_err(|_| ArenaError::IdExhausted)?;
        let relation = relation.into();
        self.relations.insert(relation_id, relation);
        Ok(relation_id)
    }

    pub fn resolve(&self, relation_id: RelationId) -> Result<&Relation, ArenaError> {
        self.relations
            .get(&relation_id)
            .ok_or(ArenaError::RelationNotFound(relation_id))
    }

    pub fn resolve_as<T>(&self, relation_id: RelationId) -> Result<&T, AnalyzerError>
    where
        for<'a> &'a T: TryFrom<&'a Relation, Error = AnalyzerError>,
    {
        let relation = self.resolve(relation_id)?;
        <&T>::try_from(relation)
    }
}
