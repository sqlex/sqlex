use crate::arena::RelationId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOp {
    Union,
    Intersect,
    Except,
}

#[derive(Debug, Clone)]
pub struct SetOperationRelation {
    pub left: RelationId,
    pub right: RelationId,
    pub op: SetOp,
    pub all: bool,
}

impl SetOperationRelation {
    pub fn new(left: RelationId, right: RelationId, op: SetOp, all: bool) -> Self {
        Self {
            left,
            right,
            op,
            all,
        }
    }
}
