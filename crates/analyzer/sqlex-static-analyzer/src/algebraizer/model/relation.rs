use crate::algebraizer::model::{
    expression::Expression,
    schema::{OutputSchema, ProjectionColumn, SortKey},
};

#[derive(Debug, Clone)]
pub(crate) enum JoinKind {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

#[derive(Debug, Clone)]
pub(crate) enum SetOp {
    Union,
    Intersect,
    Except,
}

#[derive(Debug, Clone)]
pub(crate) struct ScanNode {
    pub(crate) table: String,
    pub(crate) schema: OutputSchema,
}

#[derive(Debug, Clone)]
pub(crate) struct ValuesNode {
    pub(crate) schema: OutputSchema,
}

#[derive(Debug, Clone)]
pub(crate) struct SelectionNode {
    pub(crate) input: Box<Relation>,
    pub(crate) condition: Expression,
    pub(crate) schema: OutputSchema,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectionNode {
    pub(crate) input: Box<Relation>,
    pub(crate) columns: Vec<ProjectionColumn>,
    pub(crate) schema: OutputSchema,
}

#[derive(Debug, Clone)]
pub(crate) struct AggregationNode {
    pub(crate) input: Box<Relation>,
    pub(crate) group_by: Vec<ProjectionColumn>,
    pub(crate) aggregates: Vec<ProjectionColumn>,
    pub(crate) schema: OutputSchema,
}

#[derive(Debug, Clone)]
pub(crate) struct WindowNode {
    pub(crate) input: Box<Relation>,
    pub(crate) window_exprs: Vec<ProjectionColumn>,
    pub(crate) schema: OutputSchema,
}

#[derive(Debug, Clone)]
pub(crate) struct DistinctNode {
    pub(crate) input: Box<Relation>,
    pub(crate) schema: OutputSchema,
}

#[derive(Debug, Clone)]
pub(crate) struct SortNode {
    pub(crate) input: Box<Relation>,
    pub(crate) keys: Vec<SortKey>,
    pub(crate) schema: OutputSchema,
}

#[derive(Debug, Clone)]
pub(crate) struct LimitNode {
    pub(crate) input: Box<Relation>,
    pub(crate) limit: Option<u64>,
    pub(crate) offset: Option<u64>,
    pub(crate) schema: OutputSchema,
}

#[derive(Debug, Clone)]
pub(crate) struct AliasNode {
    pub(crate) input: Box<Relation>,
    pub(crate) schema: OutputSchema,
}

#[derive(Debug, Clone)]
pub(crate) struct JoinNode {
    pub(crate) left: Box<Relation>,
    pub(crate) right: Box<Relation>,
    pub(crate) kind: JoinKind,
    pub(crate) schema: OutputSchema,
}

#[derive(Debug, Clone)]
pub(crate) struct SetOpNode {
    pub(crate) left: Box<Relation>,
    pub(crate) right: Box<Relation>,
    pub(crate) op: SetOp,
    pub(crate) all: bool,
    pub(crate) schema: OutputSchema,
}

#[derive(Debug, Clone)]
pub(crate) enum Relation {
    Scan(ScanNode),
    Values(ValuesNode),
    Selection(SelectionNode),
    Projection(ProjectionNode),
    Aggregation(AggregationNode),
    Window(WindowNode),
    Distinct(DistinctNode),
    Sort(SortNode),
    Limit(LimitNode),
    Alias(AliasNode),
    Join(JoinNode),
    SetOperation(SetOpNode),
}

impl Relation {
    pub(crate) fn output_schema(&self) -> &OutputSchema {
        match self {
            Relation::Scan(node) => &node.schema,
            Relation::Values(node) => &node.schema,
            Relation::Selection(node) => &node.schema,
            Relation::Projection(node) => &node.schema,
            Relation::Aggregation(node) => &node.schema,
            Relation::Window(node) => &node.schema,
            Relation::Distinct(node) => &node.schema,
            Relation::Sort(node) => &node.schema,
            Relation::Limit(node) => &node.schema,
            Relation::Alias(node) => &node.schema,
            Relation::Join(node) => &node.schema,
            Relation::SetOperation(node) => &node.schema,
        }
    }
}
