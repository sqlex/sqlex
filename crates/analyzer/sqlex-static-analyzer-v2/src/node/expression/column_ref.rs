use sqlparser::ast::Ident;

use crate::arena::RelationId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotId(pub(crate) u32);

#[derive(Debug, Clone)]
pub enum ColumnRef {
    Unresolved(Vec<Ident>),
    Resolved {
        relation_id: RelationId,
        slot_id: SlotId,
    },
}

impl ColumnRef {
    pub fn unresolved(parts: Vec<Ident>) -> Self {
        Self::Unresolved(parts)
    }

    pub fn resolved(relation_id: RelationId, slot_id: SlotId) -> Self {
        Self::Resolved {
            relation_id,
            slot_id,
        }
    }
}
