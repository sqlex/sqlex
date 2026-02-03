//! Query builder for converting SQL to PlanNode trees
//!
//! Provides BuildContext for constructing PlanNode trees from SQL.

use std::collections::HashMap;

use sqlex_analyzer::AnalyzerError;
use sqlex_common::DataType;
use sqlparser::{
    ast::{
        Expr, Function, FunctionArg, FunctionArgExpr, FunctionArguments, Ident, JoinConstraint,
        JoinOperator, ObjectName, Query, Select, SelectItem, SetExpr, Statement, TableFactor,
        TableWithJoins, Value,
    },
    parser::Parser,
};

use super::{
    nodes::*,
    nullability,
    plan::{JoinCondition, JoinKind, OrderByExpr, PlanNode, ProjectColumn, SetOp, TypedExpr},
    types,
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

/// Resolved column information (internal)
#[derive(Debug, Clone)]
pub(crate) struct ResolvedColumn {
    pub(crate) name: String,
    pub(crate) data_type: DataType,
    pub(crate) nullable: bool,
    pub(crate) source_alias: Option<String>,
}

impl<'a> BuildContext<'a> {
    /// Create a new build context with the given schema.
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            cte_scope: HashMap::new(),
        }
    }

    /// Get CTE context for LogicalNode calls
    fn get_cte_context(&self) -> super::plan::CTEContext {
        self.cte_scope
            .iter()
            .map(|(name, cte)| {
                let cols = cte
                    .columns
                    .iter()
                    .map(|rc| super::plan::PlanNodeColumn {
                        name: rc.name.clone(),
                        data_type: rc.data_type.clone(),
                        nullability: rc.nullable,
                        origin_table: rc.source_alias.clone(),
                        origin_column: None,
                    })
                    .collect();
                (name.clone(), cols)
            })
            .collect()
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
                            plan: Box::new(ValuesNode {
                                rows: vec![],
                                column_names: vec![],
                            }), // Placeholder
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
            plan = Box::new(SortNode {
                input: plan,
                order_by: order_by_exprs,
            });
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

            plan = Box::new(LimitNode {
                input: plan,
                limit,
                offset,
            });
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

                Ok(Box::new(SetOperationNode {
                    op: set_op,
                    all,
                    left: left_plan,
                    right: right_plan,
                }))
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

                Ok(Box::new(ValuesNode {
                    rows: rules_rows,
                    column_names,
                }))
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
            plan = Box::new(FilterNode {
                input: plan,
                predicate: Box::new(predicate),
            });
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
            plan = Box::new(DistinctNode { input: plan });
        }

        // Wrap in Project for projection
        plan = Box::new(ProjectNode {
            input: plan,
            columns: project_cols,
        });

        Ok(plan)
    }

    /// Build PlanNode from FROM clause
    fn build_from(&mut self, from: &[TableWithJoins]) -> Result<(Box<dyn PlanNode>, Scope)> {
        if from.is_empty() {
            return Ok((
                Box::new(ValuesNode {
                    rows: vec![],
                    column_names: vec![],
                }),
                Scope::default(),
            ));
        }

        let mut plan: Option<Box<dyn PlanNode>> = None;
        let mut scope = Scope::default();

        for item in from {
            let (relation, item_scope) = self.build_table_with_joins(item)?;

            if let Some(current_plan) = plan {
                // Implicit cross join for comma-separated tables
                plan = Some(Box::new(JoinNode {
                    kind: JoinKind::Cross,
                    left: current_plan,
                    right: relation,
                    condition: None,
                }));
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
            let mut merged_scope = scope.clone();

            // Determine nullability for JOIN columns
            let nullability_result = nullability::join_nullability(
                join_kind,
                plan.as_ref(),
                right_plan.as_ref(),
                &condition,
                self.schema,
            );

            // Apply nullability to left side tables if needed
            if let nullability::ColumnNullability::ForceNullable(side) = nullability_result {
                if side == nullability::JoinSide::Left || side == nullability::JoinSide::Both {
                    for (_, cols) in merged_scope.tables.iter_mut() {
                        for col in cols.iter_mut() {
                            col.nullable = true;
                        }
                    }
                }
            }

            // Apply nullability to right side tables if needed
            let make_right_nullable = match nullability_result {
                nullability::ColumnNullability::ForceNullable(side) => {
                    side == nullability::JoinSide::Right || side == nullability::JoinSide::Both
                },
                _ => false,
            };

            for (table_name, mut cols) in right_scope.tables {
                if make_right_nullable {
                    for col in cols.iter_mut() {
                        col.nullable = true;
                    }
                }
                merged_scope.add_table(table_name, cols);
            }

            scope = merged_scope;

            plan = Box::new(JoinNode {
                kind: join_kind,
                left: plan,
                right: right_plan,
                condition,
            });
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
                    let plan = Box::new(CTERefNode {
                        name: table_name.clone(),
                        alias: Some(effective_alias.clone()),
                    });

                    let mut scope = Scope::default();
                    let cols = cte
                        .columns
                        .iter()
                        .map(|c| ResolvedColumn {
                            name: c.name.clone(),
                            data_type: c.data_type.clone(),
                            nullable: c.nullable,
                            source_alias: Some(effective_alias.clone()),
                        })
                        .collect();
                    scope.add_table(effective_alias, cols);

                    return Ok((plan, scope));
                }

                let table_def = self.schema.tables.get(&table_name).ok_or_else(|| {
                    AnalyzerError::AnalysisError(format!("Table {} not found", table_name))
                })?;

                let columns: Vec<ResolvedColumn> = table_def
                    .columns
                    .iter()
                    .map(|c| ResolvedColumn {
                        name: c.name.clone(),
                        data_type: c.data_type.clone(),
                        nullable: c.nullable,
                        source_alias: Some(effective_alias.clone()),
                    })
                    .collect();

                let mut scope = Scope::default();
                scope.add_table(effective_alias.clone(), columns);

                let plan = Box::new(TableScanNode {
                    table: table_name,
                    alias: Some(effective_alias),
                });

                Ok((plan, scope))
            },
            TableFactor::Derived {
                lateral: _,
                alias,
                subquery,
                ..
            } => {
                let (sub_plan, sub_scope) = self.build_plan(subquery).and_then(|p| {
                    let scope = self.extract_scope_from_plan(p.as_ref())?;
                    Ok((p, scope))
                })?;

                let effective_alias = alias.as_ref().map(|a| a.name.value.clone()).ok_or(
                    AnalyzerError::AnalysisError("Subquery must have an alias".to_string()),
                )?;

                let mut derived_scope = Scope::default();
                let mut resolved_cols = Vec::new();

                for cols in sub_scope.tables.values() {
                    for col in cols {
                        resolved_cols.push(ResolvedColumn {
                            name: col.name.clone(),
                            data_type: col.data_type.clone(),
                            nullable: col.nullable,
                            source_alias: Some(effective_alias.clone()),
                        });
                    }
                }
                derived_scope.add_table(effective_alias.clone(), resolved_cols);

                Ok((
                    Box::new(SubqueryNode {
                        query: sub_plan,
                        alias: effective_alias,
                    }),
                    derived_scope,
                ))
            },
            _ => todo!("handle other table factors"),
        }
    }

    /// Build a TypedExpr from an expression
    fn build_typed_expr(&mut self, expr: &Expr, scope: &Scope) -> Result<TypedExpr> {
        let (data_type, nullable) = match expr {
            Expr::Identifier(ident) => {
                let col = scope.resolve_column(None, &ident.value)?;
                (col.data_type, col.nullable)
            },
            Expr::CompoundIdentifier(idents) => {
                if idents.len() == 2 {
                    let col = scope.resolve_column(Some(&idents[0].value), &idents[1].value)?;
                    (col.data_type, col.nullable)
                } else {
                    return Err(AnalyzerError::AnalysisError(
                        "Deep compound identifiers not supported".to_string(),
                    ));
                }
            },
            Expr::Value(v) => {
                let dt = self.infer_value_type(v);
                let nullable = matches!(v, Value::Null);
                (dt, nullable)
            },
            Expr::BinaryOp { left, op, right } => {
                let l = self.build_typed_expr(left, scope)?;
                let r = self.build_typed_expr(right, scope)?;
                let dt = types::binary_op_type(l.data_type, op.clone(), r.data_type);
                let nullable = nullability::infer_binary_op_nullability(l.nullable, r.nullable);
                (dt, nullable)
            },
            Expr::UnaryOp { op, expr } => {
                let e = self.build_typed_expr(expr, scope)?;
                let dt = types::unary_op_type(*op, e.data_type);
                (dt, e.nullable)
            },
            Expr::Function(func) => {
                // Check if it's NULLIF - NULLIF always returns nullable
                let name_upper = name_to_string(&func.name).to_uppercase();
                if name_upper == "NULLIF" {
                    let (dt, _) = self.infer_function_type(func, scope)?;
                    (dt, nullability::infer_nullif_nullability())
                } else {
                    self.infer_function_type(func, scope)?
                }
            },
            Expr::Case {
                operand: _,
                conditions: _,
                results,
                else_result,
            } => {
                // CASE expression type and nullability
                // Type: use first result branch
                let first_result = results.first().ok_or(AnalyzerError::AnalysisError(
                    "CASE expression has no THEN branches".to_string(),
                ))?;
                let first_typed = self.build_typed_expr(first_result, scope)?;
                let data_type = first_typed.data_type;

                // Nullability: use helper
                let mut when_nullabilities = Vec::new();
                for result_expr in results {
                    let typed = self.build_typed_expr(result_expr, scope)?;
                    when_nullabilities.push(typed.nullable);
                }

                let mut else_nullable = None;
                if let Some(else_expr) = else_result {
                    let typed = self.build_typed_expr(else_expr, scope)?;
                    else_nullable = Some(typed.nullable);
                }

                let nullable = nullability::infer_case_nullability(
                    else_result.is_some(),
                    &when_nullabilities,
                    else_nullable,
                );

                (data_type, nullable)
            },
            Expr::Nested(e) => {
                let t = self.build_typed_expr(e, scope)?;
                (t.data_type, t.nullable)
            },
            _ => (DataType::Custom("unknown".to_string()), true),
        };
        Ok(TypedExpr::new(expr.clone(), data_type, nullable))
    }

    fn infer_value_type(&self, v: &Value) -> DataType {
        match v {
            Value::Number(num, _) => {
                // Check if it's an integer or float
                if num.contains('.') || num.contains('e') || num.contains('E') {
                    DataType::Double
                } else {
                    DataType::Int
                }
            },
            Value::SingleQuotedString(_) | Value::DoubleQuotedString(_) => DataType::Text,
            Value::Boolean(_) => DataType::Bool,
            Value::Null => DataType::Custom("NULL".to_string()),
            _ => DataType::Text,
        }
    }

    fn infer_function_type(&mut self, func: &Function, scope: &Scope) -> Result<(DataType, bool)> {
        let name = name_to_string(&func.name);

        let args_vec = if matches!(func.args, FunctionArguments::None) {
            Vec::new()
        } else if let FunctionArguments::List(ref list) = func.args {
            list.args.clone()
        } else {
            return Err(AnalyzerError::AnalysisError(
                "Subquery as function argument not supported".to_string(),
            ));
        };

        let mut typed_args = Vec::new();
        for arg in &args_vec {
            match arg {
                FunctionArg::Named {
                    arg: FunctionArgExpr::Expr(e),
                    ..
                } => {
                    typed_args.push(self.build_typed_expr(e, scope)?);
                },
                FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) => {
                    typed_args.push(self.build_typed_expr(e, scope)?);
                },
                _ => {},
            }
        }

        let arg_types: Vec<DataType> = typed_args.iter().map(|a| a.data_type.clone()).collect();

        // Try to infer aggregate function types first
        let upper_name = name.to_uppercase();
        let return_type = match upper_name.as_str() {
            "COUNT" => {
                types::aggregate_return_type(&super::plan::AggregateFunction::Count, DataType::Int)
            },
            "SUM" => {
                let input_type = arg_types.first().cloned().unwrap_or(DataType::Int);
                types::aggregate_return_type(&super::plan::AggregateFunction::Sum, input_type)
            },
            "AVG" => {
                let input_type = arg_types.first().cloned().unwrap_or(DataType::Int);
                types::aggregate_return_type(&super::plan::AggregateFunction::Avg, input_type)
            },
            "MIN" => {
                let input_type = arg_types.first().cloned().unwrap_or(DataType::Int);
                types::aggregate_return_type(&super::plan::AggregateFunction::Min, input_type)
            },
            "MAX" => {
                let input_type = arg_types.first().cloned().unwrap_or(DataType::Int);
                types::aggregate_return_type(&super::plan::AggregateFunction::Max, input_type)
            },
            _ => {
                // Fall back to regular function type inference
                types::function_return_type(&name, &arg_types)
                    .unwrap_or(DataType::Custom("unknown".to_string()))
            },
        };

        let mut nullable = typed_args.iter().any(|a| a.nullable);

        match upper_name.as_str() {
            "COALESCE" => {
                let nullabilities: Vec<bool> = typed_args.iter().map(|a| a.nullable).collect();
                nullable = nullability::infer_coalesce_nullability(&nullabilities);
            },
            "SUM" | "AVG" | "MIN" | "MAX" | "LEAD" | "LAG" | "FIRST_VALUE" | "LAST_VALUE"
            | "NTH_VALUE" => {
                nullable = true;
            },
            "COUNT" | "ROW_NUMBER" | "RANK" | "DENSE_RANK" | "NTILE" => {
                nullable = false;
            },
            _ => {
                // Default: nullable if any arg is nullable
            },
        }
        Ok((return_type, nullable))
    }

    // Helper to get scope from a plan node (re-deriving it)
    fn extract_scope_from_plan(&self, plan: &dyn PlanNode) -> Result<Scope> {
        let mut scope = Scope::default();
        let cte_ctx = self.get_cte_context();
        let cols = plan.columns(self.schema, &cte_ctx);

        let mut columns_by_table: std::collections::HashMap<String, Vec<ResolvedColumn>> =
            std::collections::HashMap::new();

        for col in cols {
            let resolved_col = ResolvedColumn {
                name: col.name,
                data_type: col.data_type,
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

        // Apply column aliases if provided
        if !column_aliases.is_empty() {
            if src_cols.len() != column_aliases.len() {
                return Err(AnalyzerError::AnalysisError(format!(
                    "CTE {} column count mismatch",
                    cte_name
                )));
            }
            Ok(src_cols
                .iter()
                .zip(column_aliases)
                .map(|(c, alias)| ResolvedColumn {
                    name: alias.name.value.clone(),
                    data_type: c.data_type.clone(),
                    nullable: c.nullable,
                    source_alias: Some(cte_name.to_string()),
                })
                .collect())
        } else {
            Ok(src_cols
                .iter()
                .map(|c| ResolvedColumn {
                    name: c.name.clone(),
                    data_type: c.data_type.clone(),
                    nullable: c.nullable,
                    source_alias: Some(cte_name.to_string()),
                })
                .collect())
        }
    }
}

/// Scope for column resolution
#[derive(Debug, Default, Clone)]
pub struct Scope {
    /// Available tables/aliases and their columns
    pub tables: HashMap<String, Vec<ResolvedColumn>>,
}

impl Scope {
    /// Resolve a column reference
    pub fn resolve_column(
        &self,
        table_alias: Option<&str>,
        col_name: &str,
    ) -> Result<ResolvedColumn> {
        if let Some(alias) = table_alias {
            if let Some(cols) = self.tables.get(alias) {
                if let Some(col) = cols.iter().find(|c| c.name == col_name) {
                    return Ok(col.clone());
                }
            }
            Err(AnalyzerError::AnalysisError(format!(
                "Column {}.{} not found",
                alias, col_name
            )))
        } else {
            let mut found = None;
            for cols in self.tables.values() {
                if let Some(col) = cols.iter().find(|c| c.name == col_name) {
                    if found.is_some() {
                        return Err(AnalyzerError::AnalysisError(format!(
                            "Ambiguous column {}",
                            col_name
                        )));
                    }
                    found = Some(col.clone());
                }
            }
            found.ok_or_else(|| {
                AnalyzerError::AnalysisError(format!("Column {} not found", col_name))
            })
        }
    }

    /// Add a table to the scope
    pub fn add_table(&mut self, alias: String, columns: Vec<ResolvedColumn>) {
        self.tables.insert(alias, columns);
    }

    pub fn merge(&mut self, other: Scope) {
        for (k, v) in other.tables {
            self.tables.insert(k, v);
        }
    }
}

/// Helper to convert ObjectName to String
fn name_to_string(name: &ObjectName) -> String {
    name.0
        .iter()
        .map(|ident| ident.value.clone())
        .collect::<Vec<_>>()
        .join(".")
}
