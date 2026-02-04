use super::arena::ArenaId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TableId(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColumnId(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprId(pub(crate) usize);

impl ArenaId for TableId {
    fn from_usize(value: usize) -> Self {
        TableId(value)
    }

    fn into_usize(self) -> usize {
        self.0
    }
}

impl ArenaId for ColumnId {
    fn from_usize(value: usize) -> Self {
        ColumnId(value)
    }

    fn into_usize(self) -> usize {
        self.0
    }
}

impl ArenaId for ExprId {
    fn from_usize(value: usize) -> Self {
        ExprId(value)
    }

    fn into_usize(self) -> usize {
        self.0
    }
}
