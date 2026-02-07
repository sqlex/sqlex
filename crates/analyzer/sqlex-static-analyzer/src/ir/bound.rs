use sqlex_common::types::DataType;
use sqlparser::ast::{BinaryOperator, UnaryOperator, Value};

use crate::{
    analysis::functions::FunctionKind,
    ir::{
        arena::Arena,
        ids::{ColumnId, ExprId, TableId},
    },
};

/// Top-level bound statement that owns all arenas.
/// Subqueries share these arenas via IDs rather than owning separate copies.
#[derive(Debug, Clone)]
pub struct BoundStatement {
    pub tables: Arena<BoundTable, TableId>,
    pub columns: Arena<BoundColumn, ColumnId>,
    pub exprs: Arena<BoundExpr, ExprId>,
    pub ctes: Vec<BoundCte>,
    pub query: BoundQueryBody,
}

/// A query body without its own arenas — references the top-level BoundStatement arenas.
#[derive(Debug, Clone)]
pub struct BoundQueryBody {
    pub body: BoundSetExpr,
    pub order_by: Vec<BoundOrderBy>,
    pub limit: Option<ExprId>,
    pub offset: Option<ExprId>,
}

#[derive(Debug, Clone)]
pub struct BoundCte {
    pub name: String,
    pub columns: Vec<String>,
    pub query: BoundQueryBody,
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
    Query(Box<BoundQueryBody>),
    Values {
        rows: Vec<Vec<ExprId>>,
    },
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
    Derived { query: BoundQueryBody },
}

#[derive(Debug, Clone)]
pub struct BoundColumn {
    pub table: TableId,
    pub name: String,
    pub data_type: Option<DataType>,
    pub nullable: Option<bool>,
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
        kind: FunctionKind,
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
        subquery: BoundQueryBody,
        negated: bool,
    },
    Subquery(BoundQueryBody),
    /// Placeholder for expressions that failed to bind.
    /// Diagnostics have already been reported; this allows analysis to continue.
    Error,
}
