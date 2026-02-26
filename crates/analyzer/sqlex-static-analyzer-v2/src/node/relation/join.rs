use crate::arena::RelationId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinKind {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

#[derive(Debug, Clone)]
pub struct JoinRelation {
    pub left: RelationId,
    pub right: RelationId,
    pub kind: JoinKind,
}

impl JoinRelation {
    pub fn new(left: RelationId, right: RelationId, kind: JoinKind) -> Self {
        Self { left, right, kind }
    }
}
