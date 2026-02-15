use sqlex_common::types::DataType;

use crate::algebra::expr::RelExpr;

pub(crate) type SlotId = u32;
pub(crate) type RelationId = u32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ColumnOrigin {
    Base { table: String, column: String },
    Derived,
}

#[derive(Debug, Clone)]
pub(crate) struct BoundColumn {
    pub(crate) slot_id: SlotId,
    pub(crate) name: String,
    #[allow(dead_code)]
    pub(crate) table_alias: Option<String>,
    pub(crate) data_type: Option<DataType>,
    pub(crate) nullable: bool,
    pub(crate) origin: ColumnOrigin,
}

#[derive(Debug, Clone)]
pub(crate) struct OutputSchema {
    #[allow(dead_code)]
    pub(crate) relation_id: RelationId,
    pub(crate) columns: Vec<BoundColumn>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Visibility {
    Visible,
    Hidden,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectionColumn {
    pub(crate) expr: BoundScalarExpr,
    pub(crate) alias: Option<String>,
    #[allow(dead_code)]
    pub(crate) visibility: Visibility,
}

#[derive(Debug, Clone)]
pub(crate) struct SortKey {
    pub(crate) expr: BoundScalarExpr,
    #[allow(dead_code)]
    pub(crate) asc: bool,
    #[allow(dead_code)]
    pub(crate) nulls_first: Option<bool>,
}

#[derive(Debug, Clone)]
pub(crate) enum BoundLiteral {
    Null,
    Bool(bool),
    Int {
        value: i64,
        raw: String,
        assignment: bool,
    },
    Float(f64),
    String(String),
    Placeholder,
}

#[derive(Debug, Clone)]
pub(crate) enum BoundBinaryOp {
    Eq,
    NotEq,
    Lt,
    Lte,
    Gt,
    Gte,
    Add,
    Sub,
    Mul,
    Div,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub(crate) enum BoundUnaryOp {
    Not,
    Neg,
    Pos,
}

#[derive(Debug, Clone)]
pub(crate) enum BoundScalarExpr {
    SlotRef(SlotId),
    CorrelatedRef {
        depth: usize,
        slot_id: SlotId,
    },
    Literal(BoundLiteral),
    BinaryOp {
        left: Box<BoundScalarExpr>,
        op: BoundBinaryOp,
        right: Box<BoundScalarExpr>,
    },
    UnaryOp {
        op: BoundUnaryOp,
        expr: Box<BoundScalarExpr>,
    },
    Function {
        name: String,
        args: Vec<BoundScalarExpr>,
    },
    AggregateCall {
        name: String,
        args: Vec<BoundScalarExpr>,
        distinct: bool,
    },
    WindowCall {
        name: String,
        args: Vec<BoundScalarExpr>,
        partition_by: Vec<BoundScalarExpr>,
        order_by: Vec<SortKey>,
    },
    Cast {
        expr: Box<BoundScalarExpr>,
        target_type: DataType,
    },
    IsNull {
        expr: Box<BoundScalarExpr>,
        negated: bool,
    },
    Case {
        operand: Option<Box<BoundScalarExpr>>,
        when_clauses: Vec<(BoundScalarExpr, BoundScalarExpr)>,
        else_expr: Option<Box<BoundScalarExpr>>,
    },
    InList {
        expr: Box<BoundScalarExpr>,
        list: Vec<BoundScalarExpr>,
        negated: bool,
    },
    InSubquery {
        expr: Box<BoundScalarExpr>,
        subquery: Box<RelExpr>,
        negated: bool,
    },
    Exists {
        subquery: Box<RelExpr>,
        negated: bool,
    },
    ScalarSubquery(Box<RelExpr>),
    Placeholder,
}
