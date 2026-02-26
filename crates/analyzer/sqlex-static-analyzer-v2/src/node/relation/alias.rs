use crate::arena::RelationId;

#[derive(Debug, Clone)]
pub struct AliasRelation {
    pub input: RelationId,
    pub alias: String,
}

impl AliasRelation {
    pub fn new(input: RelationId, alias: impl Into<String>) -> Self {
        Self {
            input,
            alias: alias.into(),
        }
    }
}
