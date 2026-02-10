use crate::ir::scalar::ScalarExpr;

// ── Projection ──

#[derive(Debug, Clone)]
pub struct ProjectionColumn {
    pub expr: ScalarExpr,
    pub alias: Option<String>,
}

// ── Aggregation ──

#[derive(Debug, Clone)]
pub struct AggregateColumn {
    pub function: AggregateFunction,
    pub args: Vec<ScalarExpr>,
    pub distinct: bool,
    pub alias: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    First,
    Last,
    ArrayAgg,
    JsonArrayAgg,
    JsonObjectAgg,
    StringAgg,
    Custom(String),
}

// ── Window ──

#[derive(Debug, Clone)]
pub struct WindowColumn {
    pub function: WindowFunction,
    pub args: Vec<ScalarExpr>,
    pub partition_by: Vec<ScalarExpr>,
    pub order_by: Vec<SortKey>,
    pub alias: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowFunction {
    Aggregate(AggregateFunction),
    RowNumber,
    Rank,
    DenseRank,
    Ntile,
    PercentRank,
    CumeDist,
    Lead,
    Lag,
    FirstValue,
    LastValue,
    NthValue,
}

// ── Join ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinKind {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

#[derive(Debug, Clone)]
pub enum JoinCondition {
    On(ScalarExpr),
    Using(Vec<String>),
    Natural,
}

// ── Sort ──

#[derive(Debug, Clone)]
pub struct SortKey {
    pub expr: ScalarExpr,
    pub asc: bool,
    pub nulls_first: Option<bool>,
}

// ── Set operations ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOp {
    Union,
    Intersect,
    Except,
}

// ── Binary operators ──

// ── Operators ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    // Comparison
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Spaceship,
    // Logical
    And,
    Or,
    Xor,
    // String
    Like,
    NotLike,
    StringConcat,
    // Bitwise
    BitwiseOr,
    BitwiseAnd,
    BitwiseXor,
    BitwiseShiftLeft,
    BitwiseShiftRight,
    // Postgres-specific
    PGRegexMatch,
    PGRegexIMatch,
    PGRegexNotMatch,
    PGRegexNotIMatch,
    PGLikeMatch,
    PGILikeMatch,
    PGNotLikeMatch,
    PGNotILikeMatch,
    PGStartsWith,
    PGOverlap,
    Overlaps,
    AtAt,
    AtArrow,
    ArrowAt,
    AtQuestion,
    Question,
    QuestionAnd,
    QuestionPipe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
    Plus,
}
