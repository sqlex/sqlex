use std::sync::Arc;

use sqlparser::ast::{BinaryOperator, UnaryOperator, Value};

use crate::ir::{
    arena::Arena,
    ids::{ColumnId, ExprId, TableId},
};

#[derive(Debug, Clone)]
pub struct BoundQuery {
    pub ctes: Vec<Arc<BoundCte>>,
    pub tables: Arena<BoundTable, TableId>,
    pub columns: Arena<BoundColumn, ColumnId>,
    pub exprs: Arena<BoundExpr, ExprId>,
    pub body: BoundSetExpr,
    pub order_by: Vec<BoundOrderBy>,
    pub limit: Option<ExprId>,
    pub offset: Option<ExprId>,
}

#[derive(Debug, Clone)]
pub struct BoundCte {
    pub name: String,
    pub columns: Vec<String>,
    pub query: Box<BoundQuery>,
    pub recursive: bool,
}

#[derive(Debug, Clone)]
pub enum BoundSetExpr {
    Select(BoundSelect),
    SetOperation {
        op: BoundSetOp,
        all: bool,
        left: Box<BoundSetExpr>,
        right: Box<BoundSetExpr>,
    },
    Query(Box<BoundQuery>),
    Values {
        rows: Vec<Vec<ExprId>>,
    },
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundSetOp {
    Union,
    Intersect,
    Except,
}

#[derive(Debug, Clone)]
pub struct BoundSelect {
    pub from: Vec<BoundFromItem>,
    pub projection: Vec<BoundProjection>,
    pub selection: Option<ExprId>,
    pub group_by: Vec<ExprId>,
    pub having: Option<ExprId>,
    pub distinct: bool,
}

#[derive(Debug, Clone)]
pub struct BoundFromItem {
    pub table: TableId,
    pub joins: Vec<BoundJoin>,
}

#[derive(Debug, Clone)]
pub struct BoundJoin {
    pub kind: BoundJoinKind,
    pub table: TableId,
    pub condition: Option<BoundJoinCondition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundJoinKind {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

#[derive(Debug, Clone)]
pub enum BoundJoinCondition {
    On(ExprId),
    Using(Vec<String>),
    Natural,
}

#[derive(Debug, Clone)]
pub struct BoundProjection {
    pub expr: ExprId,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BoundOrderBy {
    pub expr: ExprId,
    pub asc: bool,
    pub nulls_first: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct BoundTable {
    pub source: BoundTableSource,
    pub alias: Option<String>,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum BoundTableSource {
    Table { name: String },
    Cte { name: String },
    Derived { query: Box<BoundQuery> },
}

#[derive(Debug, Clone)]
pub struct BoundColumn {
    pub table: TableId,
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum BoundExpr {
    Column(ColumnId),
    Literal(Value),
    Binary {
        left: ExprId,
        op: BinaryOperator,
        right: ExprId,
    },
    Unary {
        op: UnaryOperator,
        expr: ExprId,
    },
    IsNull {
        expr: ExprId,
        negated: bool,
    },
    Function {
        name: String,
        args: Vec<ExprId>,
        distinct: bool,
        over: bool,
    },
    Case {
        operand: Option<ExprId>,
        conditions: Vec<ExprId>,
        results: Vec<ExprId>,
        else_result: Option<ExprId>,
    },
    InList {
        expr: ExprId,
        list: Vec<ExprId>,
        negated: bool,
    },
    InSubquery {
        expr: ExprId,
        subquery: Box<BoundQuery>,
        negated: bool,
    },
    Subquery(Box<BoundQuery>),
    Unsupported,
}
