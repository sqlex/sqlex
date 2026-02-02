//! Query analyzer for building PlanNode trees
//!
//! Converts SQL SELECT statements into PlanNode trees
//! for type and nullability inference.

use std::collections::HashMap;

use sqlex_analyzer::{AnalyzerError, ResultSet};
use sqlparser::{
    ast::{Query, Select, SetExpr, TableFactor, TableWithJoins},
    dialect::Dialect as SqlParserDialect,
    parser::Parser,
};

use crate::{
    plan::{PlanNode, TypedExpr},
    schema::Schema,
};

type Result<T> = std::result::Result<T, AnalyzerError>;

/// Query analyzer that builds PlanNode trees from SQL
pub struct QueryAnalyzer<'a> {
    schema: &'a Schema,
    /// CTE scope for WITH clause resolution
    #[allow(dead_code)]
    cte_scope: HashMap<String, ResolvedCTE>,
}

/// Resolved CTE information
#[derive(Debug, Clone)]
pub struct ResolvedCTE {
    pub columns: Vec<ResolvedColumn>,
    pub plan: Box<PlanNode>,
}

/// Resolved column information
#[derive(Debug, Clone)]
pub struct ResolvedColumn {
    pub name: String,
    pub data_type: sqlex_common::DataType,
    pub nullable: bool,
}

#[allow(dead_code)] // Skeleton code - methods will be used when implemented
impl<'a> QueryAnalyzer<'a> {
    /// Create a new query analyzer with the given schema
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            cte_scope: HashMap::new(),
        }
    }

    /// Analyze a SQL query and return the result set metadata
    pub fn analyze(&mut self, sql: &str) -> Result<ResultSet> {
        let plan = self.build_plan_from_sql(sql)?;
        self.extract_result_set(&plan)
    }

    /// Build a PlanNode tree from SQL string
    pub fn build_plan_from_sql(&mut self, sql: &str) -> Result<PlanNode> {
        let dialect = self.get_sqlparser_dialect();
        let statements = Parser::parse_sql(dialect.as_ref(), sql)
            .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;

        if statements.len() != 1 {
            return Err(AnalyzerError::AnalysisError(
                "Expected exactly one statement".to_string(),
            ));
        }

        match &statements[0] {
            sqlparser::ast::Statement::Query(query) => self.build_plan(query),
            _ => Err(AnalyzerError::AnalysisError(
                "Expected a SELECT query".to_string(),
            )),
        }
    }

    /// Build PlanNode from parsed Query
    fn build_plan(&mut self, query: &Query) -> Result<PlanNode> {
        // Handle WITH clause first
        if query.with.is_some() {
            todo!("handle WITH clause / CTEs")
        }

        self.build_set_expr(&query.body)
    }

    /// Build PlanNode from SetExpr (handles UNION, INTERSECT, EXCEPT)
    fn build_set_expr(&mut self, set_expr: &SetExpr) -> Result<PlanNode> {
        match set_expr {
            SetExpr::Select(select) => self.build_select(select),
            SetExpr::Query(query) => self.build_plan(query),
            SetExpr::SetOperation { .. } => {
                todo!("handle UNION / INTERSECT / EXCEPT")
            },
            SetExpr::Values(_) => {
                todo!("handle VALUES clause")
            },
            _ => Err(AnalyzerError::AnalysisError(format!(
                "Unsupported set expression: {:?}",
                set_expr
            ))),
        }
    }

    /// Build PlanNode from SELECT statement
    fn build_select(&mut self, _select: &Select) -> Result<PlanNode> {
        todo!("build SELECT plan: FROM -> WHERE -> GROUP BY -> HAVING -> SELECT -> DISTINCT")
    }

    /// Build PlanNode from FROM clause
    fn build_from(&mut self, _from: &[TableWithJoins]) -> Result<PlanNode> {
        todo!("handle FROM clause with tables and joins")
    }

    /// Build PlanNode for a single table reference
    fn build_table_factor(&mut self, _table: &TableFactor) -> Result<PlanNode> {
        todo!("handle table reference, subquery, or table function")
    }

    /// Build PlanNode for JOIN
    fn build_join(&mut self, _left: PlanNode, _join: &sqlparser::ast::Join) -> Result<PlanNode> {
        todo!("handle JOIN with condition")
    }

    /// Build projection columns
    fn build_projection(
        &mut self,
        _input: PlanNode,
        _select_items: &[sqlparser::ast::SelectItem],
    ) -> Result<PlanNode> {
        todo!("handle SELECT items / projection")
    }

    /// Build a TypedExpr from an expression
    fn build_typed_expr(&mut self, _expr: &sqlparser::ast::Expr) -> Result<TypedExpr> {
        todo!("infer type and nullability for expression")
    }

    /// Extract ResultSet from the final PlanNode
    fn extract_result_set(&self, _plan: &PlanNode) -> Result<ResultSet> {
        todo!("extract column names, types, and nullability from plan")
    }

    /// Get the sqlparser dialect
    fn get_sqlparser_dialect(&self) -> Box<dyn SqlParserDialect> {
        use sqlparser::dialect::{MySqlDialect, PostgreSqlDialect, SQLiteDialect};

        match self.schema.dialect {
            crate::schema::Dialect::PostgreSQL => Box::new(PostgreSqlDialect {}),
            crate::schema::Dialect::MySQL => Box::new(MySqlDialect {}),
            crate::schema::Dialect::SQLite => Box::new(SQLiteDialect {}),
        }
    }
}

/// Scope for column resolution
#[derive(Debug, Default)]
pub struct Scope {
    /// Available tables/aliases and their columns
    pub tables: HashMap<String, Vec<ResolvedColumn>>,
}

impl Scope {
    /// Resolve a column reference
    pub fn resolve_column(&self, _table: Option<&str>, _column: &str) -> Result<ResolvedColumn> {
        todo!("resolve column from scope")
    }

    /// Add a table to the scope
    pub fn add_table(&mut self, alias: String, columns: Vec<ResolvedColumn>) {
        self.tables.insert(alias, columns);
    }
}
