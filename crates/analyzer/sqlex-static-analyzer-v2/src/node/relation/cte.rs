use crate::arena::RelationId;

#[derive(Debug, Clone)]
pub enum CteBody {
    Regular {
        relation: RelationId,
    },
    Recursive {
        seed: RelationId,
        recursive_term: RelationId,
        all: bool,
    },
}

#[derive(Debug, Clone)]
pub struct CteRelation {
    pub name: String,
    pub body: CteBody,
}

impl CteRelation {
    pub fn new(name: impl Into<String>, body: CteBody) -> Self {
        Self {
            name: name.into(),
            body,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CteRefRelation {
    pub target: RelationId,
}

impl CteRefRelation {
    pub fn new(target: RelationId) -> Self {
        Self { target }
    }
}
