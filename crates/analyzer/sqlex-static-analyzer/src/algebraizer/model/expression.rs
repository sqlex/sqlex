use sqlex_common::types::DataType;

use crate::algebraizer::model::{
    relation::Relation,
    schema::{SlotId, SortKey},
};

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
pub(crate) enum Expression {
    SlotRef(SlotId),
    CorrelatedRef {
        depth: usize,
        slot_id: SlotId,
    },
    Literal(BoundLiteral),
    BinaryOp {
        left: Box<Expression>,
        op: BoundBinaryOp,
        right: Box<Expression>,
    },
    UnaryOp {
        op: BoundUnaryOp,
        expr: Box<Expression>,
    },
    Function {
        name: String,
        args: Vec<Expression>,
    },
    AggregateCall {
        name: String,
        args: Vec<Expression>,
        distinct: bool,
    },
    WindowCall {
        name: String,
        args: Vec<Expression>,
        partition_by: Vec<Expression>,
        order_by: Vec<SortKey>,
    },
    Cast {
        expr: Box<Expression>,
        target_type: DataType,
    },
    IsNull {
        expr: Box<Expression>,
        negated: bool,
    },
    Case {
        operand: Option<Box<Expression>>,
        when_clauses: Vec<(Expression, Expression)>,
        else_expr: Option<Box<Expression>>,
    },
    InList {
        expr: Box<Expression>,
        list: Vec<Expression>,
        negated: bool,
    },
    InSubquery {
        expr: Box<Expression>,
        subquery: Box<Relation>,
        negated: bool,
    },
    Exists {
        subquery: Box<Relation>,
        negated: bool,
    },
    ScalarSubquery(Box<Relation>),
    Placeholder,
}
