//! Query builder for converting SQL to PlanNode trees
//!
//! Provides BuildContext for constructing PlanNode trees from SQL.

use std::collections::HashMap;

use sqlex_analyzer::AnalyzerError;
use sqlparser::{
    ast::{
        Expr, Ident, JoinConstraint, JoinOperator, ObjectName, Query, Select, SelectItem, SetExpr,
        Statement, TableFactor, TableWithJoins, Value,
    },
    parser::Parser,
};

use super::{
    expr::{OrderByExpr, TypedExpr},
    nodes::{
        join::{JoinCondition, JoinKind},
        project::ProjectColumn,
        set_operation::SetOp,
        *,
    },
    plan::PlanNode,
    scope::{ResolvedColumn, Scope},
};
use crate::schema::Schema;

type Result<T> = std::result::Result<T, AnalyzerError>;

/// Build context for constructing a PlanNode tree from SQL.
///
/// Each build operation consumes the context to ensure clean state.
pub struct BuildContext<'a> {
    schema: &'a Schema,
    /// CTE scope for WITH clause resolution
    cte_scope: HashMap<String, ResolvedCTE>,
}

/// Resolved CTE information (internal)
#[derive(Debug, Clone)]
pub(crate) struct ResolvedCTE {
    pub(crate) columns: Vec<ResolvedColumn>,
    #[allow(dead_code)]
    pub(crate) plan: Box<dyn PlanNode>,
}

impl<'a> BuildContext<'a> {
    /// Create a new build context with the given schema.
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            cte_scope: HashMap::new(),
        }
    }

    /// Build a PlanNode tree from SQL string.
    /// Consumes self to ensure the context is not reused.
    pub fn build(mut self, sql: &str) -> Result<Box<dyn PlanNode>> {
        let dialect = self.schema.get_sqlparser_dialect();
        let statements = Parser::parse_sql(dialect.as_ref(), sql)
            .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;

        if statements.len() != 1 {
            return Err(AnalyzerError::AnalysisError(
                "Expected exactly one statement".to_string(),
            ));
        }

        match &statements[0] {
            Statement::Query(query) => self.build_plan(query),
            _ => Err(AnalyzerError::AnalysisError(
                "Expected a SELECT query".to_string(),
            )),
        }
    }

    /// Build PlanNode from parsed Query
    fn build_plan(&mut self, query: &Query) -> Result<Box<dyn PlanNode>> {
        // Handle WITH clause first (CTEs)
        if let Some(with) = &query.with {
            for cte in &with.cte_tables {
                let cte_name = cte.alias.name.value.clone();

                // For recursive CTEs, we need to bind the CTE name BEFORE processing
                // the full CTE body, so recursive references can find it.
                // We do this by first analyzing the anchor part (left side of UNION ALL).
                if with.recursive {
                    // Extract anchor columns from the first part of the UNION
                    let anchor_cols =
                        self.extract_anchor_columns(&cte.query, &cte_name, &cte.alias.columns)?;

                    // Pre-register the CTE with anchor schema so recursive part can reference it
                    self.cte_scope.insert(
                        cte_name.clone(),
                        ResolvedCTE {
                            columns: anchor_cols,
                            plan: Box::new(ValuesNode::build(vec![], vec![])), // Placeholder
                        },
                    );
                }

                // Now analyze the full CTE query
                let cte_plan = self.build_plan(&cte.query)?;
                let scope = self.extract_scope_from_plan(cte_plan.as_ref())?;

                // Extract columns from the first table in the scope (usually "Default")
                let src_cols = scope.tables.values().next().ok_or_else(|| {
                    AnalyzerError::AnalysisError("CTE query has no columns".to_string())
                })?;

                // Handle column aliases if provided
                if !cte.alias.columns.is_empty() {
                    if src_cols.len() != cte.alias.columns.len() {
                        return Err(AnalyzerError::AnalysisError(format!(
                            "CTE {} column count mismatch",
                            cte_name
                        )));
                    }

                    let new_cols = src_cols
                        .iter()
                        .zip(&cte.alias.columns)
                        .map(|(c, alias)| ResolvedColumn {
                            name: alias.name.value.clone(),
                            data_type: c.data_type.clone(),
                            nullable: c.nullable,
                            source_alias: Some(cte_name.clone()),
                        })
                        .collect();

                    self.cte_scope.insert(
                        cte_name.clone(),
                        ResolvedCTE {
                            columns: new_cols,
                            plan: cte_plan,
                        },
                    );
                } else {
                    // No aliases provided, use inferred names
                    let new_cols = src_cols
                        .iter()
                        .map(|c| ResolvedColumn {
                            name: c.name.clone(),
                            data_type: c.data_type.clone(),
                            nullable: c.nullable,
                            source_alias: Some(cte_name.clone()),
                        })
                        .collect();

                    self.cte_scope.insert(
                        cte_name.clone(),
                        ResolvedCTE {
                            columns: new_cols,
                            plan: cte_plan,
                        },
                    );
                }
            }
        }

        let mut plan = self.build_set_expr(&query.body)?;

        // ORDER BY
        if let Some(order_by) = &query.order_by {
            let scope = self.extract_scope_from_plan(plan.as_ref())?;
            let mut order_by_exprs = Vec::new();
            for ob in &order_by.exprs {
                let expr = self.build_typed_expr(&ob.expr, &scope)?;
                order_by_exprs.push(OrderByExpr {
                    expr,
                    asc: ob.asc.unwrap_or(true),
                    nulls_first: ob.nulls_first,
                });
            }
            plan = Box::new(SortNode::build(plan, order_by_exprs));
        }

        // LIMIT / OFFSET
        if query.limit.is_some() || query.offset.is_some() {
            let limit = if let Some(l) = &query.limit {
                match l {
                    Expr::Value(Value::Number(n, _)) => Some(n.parse().unwrap_or(0)),
                    _ => None, // Only constant limit supported
                }
            } else {
                None
            };
            let offset = if let Some(o) = &query.offset {
                match &o.value {
                    Expr::Value(Value::Number(n, _)) => Some(n.parse().unwrap_or(0)),
                    _ => None,
                }
            } else {
                None
            };

            plan = Box::new(LimitNode::build(plan, limit, offset));
        }

        Ok(plan)
    }

    /// Build PlanNode from SetExpr (handles SELECT, UNION, etc.)
    fn build_set_expr(&mut self, set_expr: &SetExpr) -> Result<Box<dyn PlanNode>> {
        match set_expr {
            SetExpr::Select(select) => self.build_select(select),
            SetExpr::Query(query) => self.build_plan(query),
            SetExpr::SetOperation {
                op,
                left,
                right,
                set_quantifier,
            } => {
                let left_plan = self.build_set_expr(left)?;
                let right_plan = self.build_set_expr(right)?;

                use sqlparser::ast::SetOperator;

                let set_op = match op {
                    SetOperator::Union => SetOp::Union,
                    SetOperator::Intersect => SetOp::Intersect,
                    SetOperator::Except => SetOp::Except,
                    _ => {
                        return Err(AnalyzerError::AnalysisError(
                            "Unsupported set operator".to_string(),
                        ));
                    },
                };

                use sqlparser::ast::SetQuantifier;
                let all = matches!(
                    set_quantifier,
                    SetQuantifier::All | SetQuantifier::AllByName
                );

                Ok(Box::new(SetOperationNode::build(
                    set_op, all, left_plan, right_plan,
                )))
            },
            SetExpr::Values(values) => {
                let mut rules_rows = Vec::new();
                let mut num_cols = 0;
                let scope = Scope::default();

                for (row_idx, row) in values.rows.iter().enumerate() {
                    let mut typed_row = Vec::new();
                    for expr in row {
                        typed_row.push(self.build_typed_expr(expr, &scope)?);
                    }

                    if row_idx == 0 {
                        num_cols = typed_row.len();
                    } else if typed_row.len() != num_cols {
                        return Err(AnalyzerError::AnalysisError(format!(
                            "VALUES clause has mismatched column counts: row {} has {}, expected {}",
                            row_idx + 1,
                            typed_row.len(),
                            num_cols
                        )));
                    }

                    rules_rows.push(typed_row);
                }

                if num_cols == 0 {
                    return Err(AnalyzerError::AnalysisError(
                        "VALUES clause must have at least one column".to_string(),
                    ));
                }

                // Generate default column names: column1, column2, ...
                let column_names = (1..=num_cols).map(|i| format!("column{}", i)).collect();

                Ok(Box::new(ValuesNode::build(rules_rows, column_names)))
            },
            _ => Err(AnalyzerError::AnalysisError(format!(
                "Unsupported set expression: {:?}",
                set_expr
            ))),
        }
    }

    /// Build PlanNode from SELECT statement
    fn build_select(&mut self, select: &Select) -> Result<Box<dyn PlanNode>> {
        // 1. FROM clause
        let (mut plan, scope) = self.build_from(&select.from)?;

        // 2. WHERE clause
        if let Some(selection) = &select.selection {
            let predicate = self.build_typed_expr(selection, &scope)?;
            plan = Box::new(FilterNode::build(plan, Box::new(predicate)));
        }

        // 3. GROUP BY
        // 4. HAVING
        // (Skipping for now to focus on simple SELECT)

        // 5. Projection (SELECT list)
        let mut project_cols = Vec::new();
        let mut new_scope_cols = Vec::new();

        for item in &select.projection {
            match item {
                SelectItem::UnnamedExpr(expr) => {
                    let typed = self.build_typed_expr(expr, &scope)?;
                    let name = match expr {
                        Expr::Identifier(ids) => ids.value.clone(),
                        Expr::CompoundIdentifier(ids) => ids.last().unwrap().value.clone(),
                        _ => format!("col_{}", project_cols.len()),
                    };
                    new_scope_cols.push(ResolvedColumn {
                        name: name.clone(),
                        data_type: typed.data_type.clone(),
                        nullable: typed.nullable,
                        source_alias: None,
                    });
                    project_cols.push(ProjectColumn {
                        alias: Some(name),
                        expr: typed,
                    });
                },
                SelectItem::ExprWithAlias { expr, alias } => {
                    let typed = self.build_typed_expr(expr, &scope)?;
                    new_scope_cols.push(ResolvedColumn {
                        name: alias.value.clone(),
                        data_type: typed.data_type.clone(),
                        nullable: typed.nullable,
                        source_alias: None,
                    });
                    project_cols.push(ProjectColumn {
                        alias: Some(alias.value.clone()),
                        expr: typed,
                    });
                },
                SelectItem::Wildcard(_options) => {
                    // Expand wildcard
                    for cols in scope.tables.values() {
                        for col in cols {
                            let expr = if let Some(alias) = &col.source_alias {
                                Expr::CompoundIdentifier(vec![
                                    Ident::new(alias.clone()),
                                    Ident::new(&col.name),
                                ])
                            } else {
                                Expr::Identifier(Ident::new(&col.name))
                            };

                            let typed = TypedExpr::new(expr, col.data_type.clone(), col.nullable);
                            new_scope_cols.push(col.clone());
                            project_cols.push(ProjectColumn {
                                alias: Some(col.name.clone()),
                                expr: typed,
                            });
                        }
                    }
                },
                SelectItem::QualifiedWildcard(obj_name, _opts) => {
                    let table_alias = name_to_string(obj_name);
                    if let Some(cols) = scope.tables.get(&table_alias) {
                        for col in cols {
                            let expr = Expr::CompoundIdentifier(vec![
                                Ident::new(&table_alias),
                                Ident::new(&col.name),
                            ]);
                            let typed = TypedExpr::new(expr, col.data_type.clone(), col.nullable);
                            new_scope_cols.push(col.clone());
                            project_cols.push(ProjectColumn {
                                alias: Some(col.name.clone()),
                                expr: typed,
                            });
                        }
                    } else {
                        return Err(AnalyzerError::AnalysisError(format!(
                            "Table alias {} not found",
                            table_alias
                        )));
                    }
                },
            }
        }

        // 6. DISTINCT
        if select.distinct.is_some() {
            plan = Box::new(DistinctNode::build(plan));
        }

        // Wrap in Project for projection
        plan = Box::new(ProjectNode::build(plan, project_cols));

        Ok(plan)
    }

    /// Build PlanNode from FROM clause
    fn build_from(&mut self, from: &[TableWithJoins]) -> Result<(Box<dyn PlanNode>, Scope)> {
        if from.is_empty() {
            return Ok((
                Box::new(ValuesNode::build(vec![], vec![])),
                Scope::default(),
            ));
        }

        let mut plan: Option<Box<dyn PlanNode>> = None;
        let mut scope = Scope::default();

        for item in from {
            let (relation, item_scope) = self.build_table_with_joins(item)?;

            if let Some(current_plan) = plan {
                // Implicit cross join for comma-separated tables
                plan = Some(Box::new(JoinNode::build(
                    self.schema,
                    current_plan,
                    relation,
                    JoinKind::Cross,
                    None,
                )));
            } else {
                plan = Some(relation);
            }
            scope.merge(item_scope);
        }

        Ok((plan.unwrap(), scope))
    }

    fn build_table_with_joins(
        &mut self,
        table_with_joins: &TableWithJoins,
    ) -> Result<(Box<dyn PlanNode>, Scope)> {
        let (mut plan, mut scope) = self.build_table_factor(&table_with_joins.relation)?;

        for join in &table_with_joins.joins {
            let (right_plan, right_scope) = self.build_table_factor(&join.relation)?;

            let (join_kind, condition) = match &join.join_operator {
                JoinOperator::Inner(constraint) => (
                    JoinKind::Inner,
                    Some(self.build_join_constraint(constraint, &scope, &right_scope)?),
                ),
                JoinOperator::LeftOuter(constraint) => (
                    JoinKind::Left,
                    Some(self.build_join_constraint(constraint, &scope, &right_scope)?),
                ),
                JoinOperator::RightOuter(constraint) => (
                    JoinKind::Right,
                    Some(self.build_join_constraint(constraint, &scope, &right_scope)?),
                ),
                JoinOperator::FullOuter(constraint) => (
                    JoinKind::Full,
                    Some(self.build_join_constraint(constraint, &scope, &right_scope)?),
                ),
                JoinOperator::CrossJoin => (JoinKind::Cross, None),
                _ => {
                    return Err(AnalyzerError::AnalysisError(
                        "Unsupported join type".to_string(),
                    ));
                },
            };

            // Merge scopes and adjust nullability based on JOIN type
            // New logic: Build the JoinNode (which calculates nullability internally)
            // then extract the updated scope from the resulting plan columns.
            let join_node = JoinNode::build(self.schema, plan, right_plan, join_kind, condition);

            scope = self.extract_scope_from_plan(&join_node)?;
            plan = Box::new(join_node);
        }

        Ok((plan, scope))
    }

    fn build_join_constraint(
        &mut self,
        constraint: &JoinConstraint,
        left_scope: &Scope,
        right_scope: &Scope,
    ) -> Result<JoinCondition> {
        let mut combined_scope = left_scope.clone();
        combined_scope.merge(right_scope.clone());

        match constraint {
            JoinConstraint::On(expr) => {
                let typed = self.build_typed_expr(expr, &combined_scope)?;
                Ok(JoinCondition::On(Box::new(typed)))
            },
            JoinConstraint::Using(idents) => Ok(JoinCondition::Using(
                idents.iter().map(name_to_string).collect(),
            )),
            JoinConstraint::Natural => Ok(JoinCondition::Natural),
            JoinConstraint::None => {
                panic!("Constraints None shouldn't happen for Inner/Outer join")
            },
        }
    }

    /// Build PlanNode for a single table reference
    fn build_table_factor(&mut self, table: &TableFactor) -> Result<(Box<dyn PlanNode>, Scope)> {
        match table {
            TableFactor::Table { name, alias, .. } => {
                let table_name = name_to_string(name);
                let effective_alias = alias
                    .as_ref()
                    .map(|a| a.name.value.clone())
                    .unwrap_or_else(|| table_name.clone());

                // Check CTEs first
                if let Some(cte) = self.cte_scope.get(&table_name) {
                    let cte_cols = cte
                        .columns
                        .iter()
                        .map(|c| super::plan::PlanNodeColumn {
                            name: c.name.clone(),
                            data_type: c.data_type.clone(),
                            nullability: c.nullable,
                            origin_table: c.source_alias.clone(),
                            origin_column: None,
                        })
                        .collect();

                    let plan = CTERefNode::build(table_name, Some(effective_alias), cte_cols);
                    let scope = self.extract_scope_from_plan(&plan)?;

                    return Ok((Box::new(plan), scope));
                }

                let plan = TableScanNode::build(self.schema, table_name, Some(effective_alias))?;
                let scope = self.extract_scope_from_plan(&plan)?;

                Ok((Box::new(plan), scope))
            },
            TableFactor::Derived {
                alias, subquery, ..
            } => {
                let sub_plan = self.build_plan(subquery)?;
                let effective_alias = alias.as_ref().map(|a| a.name.value.clone()).ok_or(
                    AnalyzerError::AnalysisError("Subquery must have an alias".to_string()),
                )?;

                let plan = SubqueryNode::build(sub_plan, effective_alias);
                let scope = self.extract_scope_from_plan(&plan)?;
                Ok((Box::new(plan), scope))
            },
            _ => todo!("handle other table factors"),
        }
    }

    /// Build a TypedExpr from an expression
    fn build_typed_expr(&mut self, expr: &Expr, scope: &Scope) -> Result<TypedExpr> {
        TypedExpr::from_expr(expr, scope)
    }

    // Helper to get scope from a plan node (re-deriving it)
    fn extract_scope_from_plan(&self, plan: &dyn PlanNode) -> Result<Scope> {
        let mut scope = Scope::default();
        let cols = plan.columns();

        let mut columns_by_table: std::collections::HashMap<String, Vec<ResolvedColumn>> =
            std::collections::HashMap::new();

        for col in cols {
            let resolved_col = ResolvedColumn {
                name: col.name.clone(),
                data_type: col.data_type.clone(),
                nullable: col.nullability,
                source_alias: col.origin_table.clone(),
            };
            let table_name = resolved_col
                .source_alias
                .clone()
                .unwrap_or_else(|| "Default".to_string());

            columns_by_table
                .entry(table_name)
                .or_default()
                .push(resolved_col);
        }

        for (table, cols) in columns_by_table {
            scope.add_table(table, cols);
        }

        Ok(scope)
    }

    /// Extract columns from the anchor (non-recursive) part of a recursive CTE
    /// For a query like: SELECT ... UNION ALL SELECT ... (recursive)
    /// We analyze only the left/anchor part to get the schema.
    fn extract_anchor_columns(
        &mut self,
        query: &Query,
        cte_name: &str,
        column_aliases: &[sqlparser::ast::TableAliasColumnDef],
    ) -> Result<Vec<ResolvedColumn>> {
        // For recursive CTE, the body is typically a SetOperation (UNION ALL)
        // We need to analyze just the anchor (left) part
        let anchor_plan = match &*query.body {
            SetExpr::SetOperation { left, .. } => {
                // Analyze only the anchor (left) part
                self.build_set_expr(left)?
            },
            // If it's not a set operation, just analyze the whole thing
            other => self.build_set_expr(other)?,
        };

        let scope = self.extract_scope_from_plan(anchor_plan.as_ref())?;
        let src_cols = scope
            .tables
            .values()
            .next()
            .ok_or(AnalyzerError::AnalysisError(
                "Anchor query has no columns".to_string(),
            ))?;

        let mut alias_map = HashMap::new();
        if !column_aliases.is_empty() {
            if src_cols.len() != column_aliases.len() {
                return Err(AnalyzerError::AnalysisError(format!(
                    "CTE {} column count mismatch in anchor: got {}, expected {}",
                    cte_name,
                    src_cols.len(),
                    column_aliases.len()
                )));
            }
            for (i, alias) in column_aliases.iter().enumerate() {
                alias_map.insert(src_cols[i].name.clone(), alias.name.value.clone());
            }
        }

        let new_cols = src_cols
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let final_name = if !column_aliases.is_empty() {
                    column_aliases[i].name.value.clone()
                } else {
                    c.name.clone() // Keep original name
                };
                ResolvedColumn {
                    name: final_name,
                    data_type: c.data_type.clone(),
                    nullable: c.nullable,
                    source_alias: Some(cte_name.to_string()),
                }
            })
            .collect();

        Ok(new_cols)
    }
}

/// Helper to extract table name from object name
fn name_to_string(name: &ObjectName) -> String {
    name.0
        .iter()
        .map(|i| i.value.clone())
        .collect::<Vec<_>>()
        .join(".")
}
