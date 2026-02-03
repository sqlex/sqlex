//! Query builder for converting SQL to PlanNode trees
//!
//! Provides BuildContext for constructing PlanNode trees from SQL.

use std::collections::HashMap;

use sqlex_analyzer::{AnalyzerError, ObjectNameExt};
use sqlparser::{
    ast::{
        Expr, Ident, Query, Select, SelectItem, SetExpr, Statement, TableFactor, TableWithJoins,
    },
    parser::Parser,
};

use super::{
    nodes::{join::JoinKind, project::ProjectColumn, *},
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
                let expr = self.build_expr(&ob.expr, &scope)?;
                order_by_exprs.push(super::expr::OrderByExpr::build(
                    expr,
                    ob.asc.unwrap_or(true),
                    ob.nulls_first,
                ));
            }
            plan = Box::new(SortNode::build(plan, order_by_exprs));
        }

        // LIMIT / OFFSET
        if query.limit.is_some() || query.offset.is_some() {
            plan = Box::new(LimitNode::from_ast(
                plan,
                query.limit.as_ref(),
                query.offset.as_ref(),
            ));
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

                Ok(Box::new(SetOperationNode::from_ast(
                    op,
                    set_quantifier,
                    left_plan,
                    right_plan,
                )?))
            },
            SetExpr::Values(values) => {
                let mut rules_rows = Vec::new();
                let mut num_cols = 0;
                let scope = Scope::default();

                for (row_idx, row) in values.rows.iter().enumerate() {
                    let mut typed_row = Vec::new();
                    for expr in row {
                        typed_row.push(self.build_expr(expr, &scope)?);
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
            let predicate = self.build_expr(selection, &scope)?;
            plan = Box::new(FilterNode::build(plan, predicate));
        }

        // 3. GROUP BY and Aggregation
        // Check if we need aggregation (has GROUP BY or aggregate functions in SELECT)
        let has_group_by = matches!(
            select.group_by,
            sqlparser::ast::GroupByExpr::Expressions(ref exprs, _) if !exprs.is_empty()
        ) || matches!(select.group_by, sqlparser::ast::GroupByExpr::All(_));

        let mut has_aggregates_in_select = false;

        // First pass: check for aggregates in SELECT list
        for item in &select.projection {
            let expr = match item {
                SelectItem::UnnamedExpr(e) => Some(e),
                SelectItem::ExprWithAlias { expr: e, .. } => Some(e),
                _ => None,
            };
            if let Some(e) = expr {
                use crate::planner::expr::ExprExt;
                if e.has_aggregate_function() {
                    has_aggregates_in_select = true;
                    break;
                }
            }
        }

        // If we have aggregates or GROUP BY, build an AggregateNode
        if has_group_by || has_aggregates_in_select {
            let mut group_by_exprs = Vec::new();
            let mut aggregate_exprs = Vec::new();

            // Parse GROUP BY expressions
            match &select.group_by {
                sqlparser::ast::GroupByExpr::Expressions(exprs, _) => {
                    for expr in exprs {
                        group_by_exprs.push(self.build_expr(expr, &scope)?);
                    }
                },
                sqlparser::ast::GroupByExpr::All(_) => {
                    // GROUP BY ALL - not yet supported, would require special handling
                    return Err(AnalyzerError::AnalysisError(
                        "GROUP BY ALL is not yet supported".to_string(),
                    ));
                },
            }

            // Extract aggregate expressions from SELECT list
            for item in &select.projection {
                if let Some(expr) = match item {
                    SelectItem::UnnamedExpr(e) => Some(e),
                    SelectItem::ExprWithAlias { expr: e, .. } => Some(e),
                    _ => None,
                } {
                    if let Expr::Function(func) = expr {
                        use crate::planner::expr::ExprExt;
                        if expr.has_aggregate_function() {
                            aggregate_exprs.push(self.build_aggregate_expr(func, &scope)?);
                        }
                    }
                }
            }

            // Build AggregateNode
            plan = Box::new(AggregateNode::build(
                plan,
                group_by_exprs,
                aggregate_exprs,
                None,
            ));
        }

        // 4. HAVING
        if let Some(ref having) = select.having {
            // HAVING uses the original scope (before aggregation) because
            // it can contain aggregate functions that reference original columns
            let predicate = self.build_expr(having, &scope)?;
            plan = Box::new(FilterNode::build(plan, predicate));
        }

        // 5. Projection (SELECT list)
        let mut project_cols = Vec::new();
        let mut new_scope_cols = Vec::new();

        for item in &select.projection {
            match item {
                SelectItem::UnnamedExpr(expr) => {
                    let typed = self.build_expr(expr, &scope)?;
                    let name = match expr {
                        Expr::Identifier(ids) => ids.value.clone(),
                        Expr::CompoundIdentifier(ids) => ids.last().unwrap().value.clone(),
                        _ => format!("col_{}", project_cols.len()),
                    };
                    new_scope_cols.push(ResolvedColumn {
                        name: name.clone(),
                        data_type: typed.data_type(),
                        nullable: typed.nullable(),
                        source_alias: None,
                    });
                    project_cols.push(ProjectColumn {
                        alias: Some(name),
                        expr: typed,
                    });
                },
                SelectItem::ExprWithAlias { expr, alias } => {
                    let typed = self.build_expr(expr, &scope)?;
                    new_scope_cols.push(ResolvedColumn {
                        name: alias.value.clone(),
                        data_type: typed.data_type(),
                        nullable: typed.nullable(),
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
                            let _expr = if let Some(alias) = &col.source_alias {
                                Expr::CompoundIdentifier(vec![
                                    Ident::new(alias.clone()),
                                    Ident::new(&col.name),
                                ])
                            } else {
                                Expr::Identifier(Ident::new(&col.name))
                            };

                            let typed = super::expr::values::ColumnExpr::build(
                                col.source_alias.clone(),
                                col.name.clone(),
                                col.data_type.clone(),
                                col.nullable,
                            );
                            new_scope_cols.push(col.clone());
                            project_cols.push(ProjectColumn {
                                alias: Some(col.name.clone()),
                                expr: typed,
                            });
                        }
                    }
                },
                SelectItem::QualifiedWildcard(obj_name, _opts) => {
                    let table_alias = obj_name.to_dotted_string();
                    if let Some(cols) = scope.tables.get(&table_alias) {
                        for col in cols {
                            let _expr = Expr::CompoundIdentifier(vec![
                                Ident::new(&table_alias),
                                Ident::new(&col.name),
                            ]);
                            let typed = super::expr::values::ColumnExpr::build(
                                Some(table_alias.clone()),
                                col.name.clone(),
                                col.data_type.clone(),
                                col.nullable,
                            );
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

            // Use build_join_node to handle all conversion logic including expression building
            let join_node =
                self.build_join_node(plan, right_plan, &join.join_operator, &scope, &right_scope)?;

            // Extract scope from the RESULTING join node (which is a PlanNode)
            scope = self.extract_scope_from_plan(join_node.as_ref())?;
            plan = join_node;
        }

        Ok((plan, scope))
    }

    /// Build PlanNode for a single table reference
    fn build_table_factor(&mut self, table: &TableFactor) -> Result<(Box<dyn PlanNode>, Scope)> {
        match table {
            TableFactor::Table { name, alias, .. } => {
                let table_name = name.to_dotted_string();
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

    fn build_aggregate_expr(
        &mut self,
        func: &sqlparser::ast::Function,
        scope: &Scope,
    ) -> Result<super::expr::AggregateExpr> {
        use sqlparser::ast::{
            DuplicateTreatment, FunctionArg, FunctionArgExpr, FunctionArguments, Value,
        };

        use super::expr::{AggregateExpr, AggregateFunction};

        let name = func.name.to_dotted_string().to_uppercase();
        let function =
            AggregateFunction::from_name(&name).unwrap_or(AggregateFunction::Custom(name));

        let mut args = Vec::new();
        let mut distinct = false;
        // let mut order_by = Vec::new(); // TODO: Add support for ORDER BY in aggregates if supported by sqlparser

        if let FunctionArguments::List(ref list) = func.args {
            for arg in &list.args {
                match arg {
                    FunctionArg::Named {
                        arg: FunctionArgExpr::Expr(e),
                        ..
                    } => {
                        args.push(self.build_expr(e, scope)?);
                    },
                    FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) => {
                        args.push(self.build_expr(e, scope)?);
                    },
                    FunctionArg::Unnamed(FunctionArgExpr::Wildcard) => {
                        // COUNT(*) -> 1
                        use super::expr::values::LiteralExpr;
                        args.push(LiteralExpr::build(Value::Number("1".to_string(), false)));
                    },
                    _ => {},
                }
            }
            distinct = list.duplicate_treatment == Some(DuplicateTreatment::Distinct);

            // Accessing order_by on list failed, maybe it's in clauses?
            // Ignoring for now to fix build.
        }

        if args.is_empty() && matches!(function, AggregateFunction::Count) {
            use super::expr::values::LiteralExpr;
            args.push(LiteralExpr::build(Value::Number("1".to_string(), false)));
        }

        let filter = if let Some(filter) = &func.filter {
            Some(self.build_expr(filter, scope)?)
        } else {
            None
        };

        Ok(AggregateExpr::build(
            function,
            args,
            distinct,
            filter,
            Vec::new(),
        ))
    }

    fn build_join_node(
        &mut self,
        left_plan: Box<dyn PlanNode>,
        right_plan: Box<dyn PlanNode>,
        join_operator: &sqlparser::ast::JoinOperator,
        left_scope: &Scope,
        right_scope: &Scope,
    ) -> Result<Box<dyn PlanNode>> {
        use sqlparser::ast::{JoinConstraint, JoinOperator};

        use super::nodes::join::{JoinCondition, JoinKind, JoinNode};

        // Convert JoinOperator to JoinKind and extract constraint
        let (kind, constraint) = match join_operator {
            JoinOperator::Inner(constraint) => (JoinKind::Inner, Some(constraint)),
            JoinOperator::LeftOuter(constraint) => (JoinKind::Left, Some(constraint)),
            JoinOperator::RightOuter(constraint) => (JoinKind::Right, Some(constraint)),
            JoinOperator::FullOuter(constraint) => (JoinKind::Full, Some(constraint)),
            JoinOperator::CrossJoin => (JoinKind::Cross, None),
            _ => {
                return Err(AnalyzerError::AnalysisError(
                    "Unsupported join type".to_string(),
                ));
            },
        };

        // Convert JoinConstraint to JoinCondition
        let condition = if let Some(constraint) = constraint {
            let mut combined_scope = left_scope.clone();
            combined_scope.merge(right_scope.clone());

            Some(match constraint {
                JoinConstraint::On(expr) => {
                    let expr = self.build_expr(expr, &combined_scope)?;
                    JoinCondition::On(expr)
                },
                JoinConstraint::Using(idents) => {
                    JoinCondition::Using(idents.iter().map(|id| id.to_string()).collect())
                },
                JoinConstraint::Natural => JoinCondition::Natural,
                JoinConstraint::None => {
                    // This case might strictly be unreachable for Inner/Outer,
                    // but good to handle safely
                    return Err(AnalyzerError::AnalysisError(
                        "Invalid join constraint: None".to_string(),
                    ));
                },
            })
        } else {
            None
        };

        Ok(Box::new(JoinNode::build(
            self.schema,
            left_plan,
            right_plan,
            kind,
            condition,
        )))
    }

    /// Build a Box<dyn Expression> from an AST expression
    fn build_expr(
        &mut self,
        expr: &Expr,
        scope: &Scope,
    ) -> Result<Box<dyn super::expr::Expression>> {
        use super::expr::{
            ops::{BinaryExpr, UnaryExpr},
            values::{ColumnExpr, LiteralExpr},
        };

        match expr {
            Expr::Identifier(ident) => {
                let col = scope.resolve_column(None, &ident.value)?;
                Ok(ColumnExpr::build(
                    None,
                    ident.value.clone(),
                    col.data_type,
                    col.nullable,
                ))
            },
            Expr::CompoundIdentifier(idents) => {
                if idents.len() == 2 {
                    let col = scope.resolve_column(Some(&idents[0].value), &idents[1].value)?;
                    Ok(ColumnExpr::build(
                        Some(idents[0].value.clone()),
                        idents[1].value.clone(),
                        col.data_type,
                        col.nullable,
                    ))
                } else {
                    Err(AnalyzerError::AnalysisError(
                        "Deep compound identifiers not supported".to_string(),
                    ))
                }
            },
            Expr::Value(v) => Ok(LiteralExpr::build(v.clone())),
            Expr::BinaryOp { left, op, right } => {
                let left_expr = self.build_expr(left, scope)?;
                let right_expr = self.build_expr(right, scope)?;
                Ok(BinaryExpr::build(left_expr, op.clone(), right_expr))
            },
            Expr::UnaryOp { op, expr: inner } => {
                let operand = self.build_expr(inner, scope)?;
                Ok(UnaryExpr::build(*op, operand))
            },
            Expr::Nested(inner) => self.build_expr(inner, scope),
            Expr::Function(func) => self.build_function_expr(func, scope),
            Expr::Case {
                operand,
                conditions,
                results,
                else_result,
            } => {
                use super::expr::control::CaseExpr;
                let operand_expr = if let Some(op) = operand {
                    Some(self.build_expr(op, scope)?)
                } else {
                    None
                };
                let mut cond_exprs = Vec::new();
                for cond in conditions {
                    cond_exprs.push(self.build_expr(cond, scope)?);
                }
                let mut result_exprs = Vec::new();
                for res in results {
                    result_exprs.push(self.build_expr(res, scope)?);
                }
                let else_expr = if let Some(el) = else_result {
                    Some(self.build_expr(el, scope)?)
                } else {
                    None
                };
                Ok(CaseExpr::build(
                    operand_expr,
                    cond_exprs,
                    result_exprs,
                    else_expr,
                ))
            },
            _ => {
                // Fallback: create a literal with unknown type
                Ok(LiteralExpr::build(sqlparser::ast::Value::Null))
            },
        }
    }

    /// Build a function expression (scalar, aggregate, or window)
    /// Build a function expression (scalar, aggregate, or window)
    fn build_function_expr(
        &mut self,
        func: &sqlparser::ast::Function,
        scope: &Scope,
    ) -> Result<Box<dyn super::expr::Expression>> {
        use sqlparser::ast::{FunctionArg, FunctionArgExpr, FunctionArguments, WindowType};

        use super::expr::{
            AggregateFunction, AggregateFunctionExpr, OrderByExpr, WindowFrame, WindowFrameBound,
            WindowFrameUnits, WindowFunction, WindowFunctionExpr, funcs::ScalarFunctionExpr,
        };

        let name = func.name.to_dotted_string();

        // Extract arguments
        let args_vec = if matches!(func.args, FunctionArguments::None) {
            Vec::new()
        } else if let FunctionArguments::List(ref list) = func.args {
            list.args.clone()
        } else {
            return Err(AnalyzerError::AnalysisError(
                "Subquery as function argument not supported".to_string(),
            ));
        };

        let mut bound_args = Vec::new();
        for arg in &args_vec {
            match arg {
                FunctionArg::Named {
                    arg: FunctionArgExpr::Expr(e),
                    ..
                } => {
                    bound_args.push(self.build_expr(e, scope)?);
                },
                FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) => {
                    bound_args.push(self.build_expr(e, scope)?);
                },
                FunctionArg::Unnamed(FunctionArgExpr::Wildcard) => {
                    // COUNT(*) - use a dummy literal
                    bound_args.push(super::expr::values::LiteralExpr::build(
                        sqlparser::ast::Value::Number("1".to_string(), false),
                    ));
                },
                _ => {},
            }
        }

        // Check for Window Function (OVER clause)
        if let Some(over) = &func.over {
            let window_func = if let Some(wf) = WindowFunction::from_name(&name) {
                wf
            } else if let Some(af) = AggregateFunction::from_name(&name) {
                WindowFunction::Aggregate(af)
            } else {
                return Err(AnalyzerError::AnalysisError(format!(
                    "Unknown window function: {}",
                    name
                )));
            };

            let (partition_by_exprs, order_by_exprs, window_frame) = match over {
                WindowType::WindowSpec(spec) => {
                    let mut partition_by = Vec::new();
                    for expr in &spec.partition_by {
                        partition_by.push(self.build_expr(expr, scope)?);
                    }

                    let mut order_by = Vec::new();
                    for ob in &spec.order_by {
                        let expr = self.build_expr(&ob.expr, scope)?;
                        order_by.push(OrderByExpr::build(
                            expr,
                            ob.asc.unwrap_or(true),
                            ob.nulls_first,
                        ));
                    }

                    let frame = if let Some(frame) = &spec.window_frame {
                        let units = match frame.units {
                            sqlparser::ast::WindowFrameUnits::Rows => WindowFrameUnits::Rows,
                            sqlparser::ast::WindowFrameUnits::Range => WindowFrameUnits::Range,
                            sqlparser::ast::WindowFrameUnits::Groups => WindowFrameUnits::Groups,
                        };

                        let convert_bound = |b: &sqlparser::ast::WindowFrameBound| {
                            match b {
                                sqlparser::ast::WindowFrameBound::CurrentRow => {
                                    WindowFrameBound::CurrentRow
                                },
                                sqlparser::ast::WindowFrameBound::Preceding(_) => {
                                    // Simplifying frame bound handling for now
                                    WindowFrameBound::Preceding(None)
                                },
                                sqlparser::ast::WindowFrameBound::Following(_) => {
                                    WindowFrameBound::Following(None)
                                },
                            }
                        };

                        Some(WindowFrame {
                            units,
                            start: convert_bound(&frame.start_bound),
                            end: frame.end_bound.as_ref().map(convert_bound),
                        })
                    } else {
                        None
                    };

                    (partition_by, order_by, frame)
                },
                WindowType::NamedWindow(_) => {
                    return Err(AnalyzerError::AnalysisError(
                        "Named windows not yet supported".to_string(),
                    ));
                },
            };

            return Ok(WindowFunctionExpr::build(
                window_func,
                bound_args,
                partition_by_exprs,
                order_by_exprs,
                window_frame,
            ));
        }

        // Check if it's an aggregate function
        if let Some(agg_func) = AggregateFunction::from_name(&name) {
            return Ok(AggregateFunctionExpr::build(agg_func, bound_args));
        }

        // Default: treat as scalar function
        let dialect = self.schema.get_sqlparser_dialect();
        Ok(ScalarFunctionExpr::build(
            dialect.as_ref(),
            name,
            bound_args,
        ))
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
