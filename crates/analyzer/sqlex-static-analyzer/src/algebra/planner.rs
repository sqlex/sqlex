use std::collections::{HashMap, HashSet};

use sqlex_analyzer::extension::DataTypeExt;
use sqlex_common::{dialect::Dialect, types::DataType};
use sqlparser::ast::{
    BinaryOperator, CeilFloorKind, DateTimeField, Expr, Function, FunctionArg, FunctionArgExpr,
    FunctionArguments, GroupByExpr, Join, JoinConstraint, JoinOperator, Select, SelectItem,
    SetExpr, SetOperator, SetQuantifier, Statement, TableFactor, UnaryOperator, Value, WindowType,
};

use crate::{
    algebra::{
        expr::{LimitNode, ProjectionNode, RelExpr, ScanNode, SelectionNode, ValuesNode},
        scalar::{
            BoundBinaryOp, BoundColumn, BoundLiteral, BoundScalarExpr, BoundUnaryOp, ColumnOrigin,
            OutputSchema, ProjectionColumn, Visibility,
        },
    },
    catalog::{
        ddl_type_map,
        model::{Catalog, TableSchema},
        normalize::{normalize_ident, normalize_object_name},
    },
    diagnostics::{Diagnostic, Phase},
    functions::registry::{FunctionCategory, FunctionRegistry},
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Algebraizer {
    dialect: Dialect,
}

#[derive(Debug, Clone)]
struct RelationScope {
    visible_names: Vec<String>,
    schema: OutputSchema,
}

#[derive(Debug, Clone)]
struct CteBinding {
    expr: RelExpr,
    exposed_schema: OutputSchema,
}

#[derive(Debug)]
struct BuildContext {
    relation_scopes: Vec<RelationScope>,
    next_relation_id: u32,
    next_slot_id: u32,
    ctes: HashMap<String, CteBinding>,
    literal_assignment_mode: bool,
}

impl BuildContext {
    fn new() -> Self {
        Self {
            relation_scopes: Vec::new(),
            next_relation_id: 1,
            next_slot_id: 1,
            ctes: HashMap::new(),
            literal_assignment_mode: false,
        }
    }

    fn allocate_relation_id(&mut self) -> u32 {
        let relation_id = self.next_relation_id;
        self.next_relation_id += 1;
        relation_id
    }

    fn allocate_slot_id(&mut self) -> u32 {
        let slot_id = self.next_slot_id;
        self.next_slot_id += 1;
        slot_id
    }

    fn current_columns(&self) -> Vec<BoundColumn> {
        self.relation_scopes
            .iter()
            .flat_map(|scope| scope.schema.columns.iter().cloned())
            .collect()
    }
}

impl Algebraizer {
    pub(crate) fn new(dialect: Dialect) -> Self {
        Self { dialect }
    }

    pub(crate) fn build(
        &self,
        statement: &Statement,
        catalog: &Catalog,
        functions: &FunctionRegistry,
    ) -> Result<RelExpr, Diagnostic> {
        let Statement::Query(query) = statement else {
            return Err(Diagnostic::new(
                "A3001",
                Phase::Algebraize,
                "only query statements are supported in analyze",
            ));
        };

        if query.order_by.is_some() {
            return Err(Diagnostic::todo(
                Phase::Algebraize,
                "top-level ORDER BY planning",
            ));
        }

        let mut context = BuildContext::new();
        if let Some(with_clause) = &query.with {
            self.register_ctes(with_clause, catalog, functions, &mut context)?;
        }
        let relational_expr = self.build_set_expr(&query.body, catalog, functions, &mut context)?;
        self.apply_top_level_limit_offset(relational_expr, query)
    }

    fn register_ctes(
        &self,
        with_clause: &sqlparser::ast::With,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &mut BuildContext,
    ) -> Result<(), Diagnostic> {
        let mut seen_names = HashSet::new();

        if with_clause.recursive {
            for cte in &with_clause.cte_tables {
                let cte_name = normalize_ident(&cte.alias.name, self.dialect);
                if !seen_names.insert(cte_name.clone()) || context.ctes.contains_key(&cte_name) {
                    return Err(Diagnostic::new(
                        "A3025",
                        Phase::Algebraize,
                        format!("duplicate CTE name: {cte_name}"),
                    ));
                }
                let binding = self.build_recursive_cte_stub(cte, catalog, context)?;
                context.ctes.insert(cte_name, binding);
            }
            return Ok(());
        }

        if matches!(self.dialect, Dialect::SQLite) {
            for cte in &with_clause.cte_tables {
                if cte.from.is_some() {
                    return Err(Diagnostic::todo(
                        Phase::Algebraize,
                        "CTE SEARCH/CYCLE clause planning",
                    ));
                }

                let cte_name = normalize_ident(&cte.alias.name, self.dialect);
                if !seen_names.insert(cte_name.clone()) || context.ctes.contains_key(&cte_name) {
                    return Err(Diagnostic::new(
                        "A3025",
                        Phase::Algebraize,
                        format!("duplicate CTE name: {cte_name}"),
                    ));
                }
                let binding = self.build_recursive_cte_stub(cte, catalog, context)?;
                context.ctes.insert(cte_name, binding);
            }

            for _ in 0..with_clause.cte_tables.len() {
                for cte in &with_clause.cte_tables {
                    let cte_name = normalize_ident(&cte.alias.name, self.dialect);
                    let mut cte_context = BuildContext {
                        relation_scopes: Vec::new(),
                        next_relation_id: context.next_relation_id,
                        next_slot_id: context.next_slot_id,
                        ctes: context.ctes.clone(),
                        literal_assignment_mode: context.literal_assignment_mode,
                    };
                    let cte_expr =
                        self.build_set_expr(&cte.query.body, catalog, functions, &mut cte_context)?;
                    context.next_relation_id = cte_context.next_relation_id;
                    context.next_slot_id = cte_context.next_slot_id;

                    let mut exposed_schema = output_schema_of(&cte_expr)?;
                    if !cte.alias.columns.is_empty() {
                        if cte.alias.columns.len() != exposed_schema.columns.len() {
                            return Err(Diagnostic::new(
                                "A3014",
                                Phase::Algebraize,
                                format!(
                                    "CTE column alias count mismatch: expected {}, got {}",
                                    exposed_schema.columns.len(),
                                    cte.alias.columns.len()
                                ),
                            ));
                        }
                        for (column, alias_column) in exposed_schema
                            .columns
                            .iter_mut()
                            .zip(cte.alias.columns.iter())
                        {
                            column.name = normalize_ident(&alias_column.name, self.dialect);
                        }
                    }

                    context.ctes.insert(
                        cte_name.clone(),
                        CteBinding {
                            expr: cte_expr,
                            exposed_schema,
                        },
                    );
                }
            }
            return Ok(());
        }

        for cte in &with_clause.cte_tables {
            if cte.from.is_some() {
                return Err(Diagnostic::todo(
                    Phase::Algebraize,
                    "CTE SEARCH/CYCLE clause planning",
                ));
            }

            let cte_name = normalize_ident(&cte.alias.name, self.dialect);
            if !seen_names.insert(cte_name.clone()) || context.ctes.contains_key(&cte_name) {
                return Err(Diagnostic::new(
                    "A3025",
                    Phase::Algebraize,
                    format!("duplicate CTE name: {cte_name}"),
                ));
            }
            let mut cte_context = BuildContext {
                relation_scopes: Vec::new(),
                next_relation_id: context.next_relation_id,
                next_slot_id: context.next_slot_id,
                ctes: context.ctes.clone(),
                literal_assignment_mode: context.literal_assignment_mode,
            };
            let cte_expr =
                self.build_set_expr(&cte.query.body, catalog, functions, &mut cte_context)?;
            context.next_relation_id = cte_context.next_relation_id;
            context.next_slot_id = cte_context.next_slot_id;

            let mut exposed_schema = output_schema_of(&cte_expr)?;
            if !cte.alias.columns.is_empty() {
                if cte.alias.columns.len() != exposed_schema.columns.len() {
                    return Err(Diagnostic::new(
                        "A3014",
                        Phase::Algebraize,
                        format!(
                            "CTE column alias count mismatch: expected {}, got {}",
                            exposed_schema.columns.len(),
                            cte.alias.columns.len()
                        ),
                    ));
                }
                for (column, alias_column) in exposed_schema
                    .columns
                    .iter_mut()
                    .zip(cte.alias.columns.iter())
                {
                    column.name = normalize_ident(&alias_column.name, self.dialect);
                }
            }

            context.ctes.insert(
                cte_name,
                CteBinding {
                    expr: cte_expr,
                    exposed_schema,
                },
            );
        }

        Ok(())
    }

    fn build_recursive_cte_stub(
        &self,
        cte: &sqlparser::ast::Cte,
        catalog: &Catalog,
        context: &mut BuildContext,
    ) -> Result<CteBinding, Diagnostic> {
        if let Some((seed_count, recursive_count)) =
            recursive_cte_set_operation_projection_counts(&cte.query.body)
        {
            if seed_count != recursive_count {
                return Err(Diagnostic::new(
                    "A3026",
                    Phase::Algebraize,
                    format!(
                        "recursive CTE term column count mismatch: seed {}, recursive {}",
                        seed_count, recursive_count
                    ),
                ));
            }
        }

        let Some(seed_select) = recursive_cte_seed_select(&cte.query.body) else {
            return Err(Diagnostic::todo(
                Phase::Algebraize,
                "recursive CTE seed extraction",
            ));
        };

        let alias_columns = &cte.alias.columns;
        if !alias_columns.is_empty() && alias_columns.len() != seed_select.projection.len() {
            return Err(Diagnostic::new(
                "A3015",
                Phase::Algebraize,
                format!(
                    "recursive CTE column alias count mismatch: expected {}, got {}",
                    seed_select.projection.len(),
                    alias_columns.len()
                ),
            ));
        }

        let mut columns = Vec::with_capacity(seed_select.projection.len());
        for (index, item) in seed_select.projection.iter().enumerate() {
            let (data_type, nullable) = projection_expr(item)
                .and_then(|expr| self.resolve_scalar_subquery_expr_type(seed_select, expr, catalog))
                .unwrap_or((DataType::Custom("unknown".to_string()), true));

            let name = if !alias_columns.is_empty() {
                normalize_ident(&alias_columns[index].name, self.dialect)
            } else {
                match item {
                    SelectItem::ExprWithAlias { alias, .. } => normalize_ident(alias, self.dialect),
                    SelectItem::UnnamedExpr(expr) => self
                        .derive_output_name(expr)
                        .unwrap_or_else(|_| format!("column_{}", index + 1)),
                    SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _) => {
                        format!("column_{}", index + 1)
                    },
                }
            };

            columns.push(BoundColumn {
                slot_id: context.allocate_slot_id(),
                name,
                table_alias: None,
                data_type: Some(data_type),
                nullable,
                origin: ColumnOrigin::Derived,
            });
        }

        let cte_name = normalize_ident(&cte.alias.name, self.dialect);
        let schema = OutputSchema {
            relation_id: context.allocate_relation_id(),
            columns,
        };
        Ok(CteBinding {
            expr: RelExpr::Scan(ScanNode {
                table: format!("__recursive_cte__{cte_name}"),
                schema: schema.clone(),
            }),
            exposed_schema: schema,
        })
    }

    fn build_set_expr(
        &self,
        set_expr: &SetExpr,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &mut BuildContext,
    ) -> Result<RelExpr, Diagnostic> {
        match set_expr {
            SetExpr::Select(select) => self.build_select(select, catalog, functions, context),
            SetExpr::Query(query) => {
                if query.with.is_some()
                    || query.order_by.is_some()
                    || query.limit.is_some()
                    || query.offset.is_some()
                {
                    return Err(Diagnostic::todo(
                        Phase::Algebraize,
                        "nested query ORDER/LIMIT/WITH planning",
                    ));
                }
                self.build_set_expr(&query.body, catalog, functions, context)
            },
            SetExpr::SetOperation {
                left,
                op,
                set_quantifier,
                right,
            } => {
                let left_expr = self.build_set_expr(left, catalog, functions, context)?;
                let right_expr = self.build_set_expr(right, catalog, functions, context)?;

                let left_schema = output_schema_of(&left_expr)?;
                let right_schema = output_schema_of(&right_expr)?;
                if left_schema.columns.len() != right_schema.columns.len() {
                    return Err(Diagnostic::new(
                        "A3019",
                        Phase::Algebraize,
                        format!(
                            "set operation column count mismatch: left {}, right {}",
                            left_schema.columns.len(),
                            right_schema.columns.len()
                        ),
                    ));
                }

                let set_op = match op {
                    SetOperator::Union => crate::algebra::expr::SetOp::Union,
                    SetOperator::Intersect => crate::algebra::expr::SetOp::Intersect,
                    SetOperator::Except | SetOperator::Minus => crate::algebra::expr::SetOp::Except,
                };
                let all = matches!(
                    set_quantifier,
                    SetQuantifier::All | SetQuantifier::AllByName
                );

                let mut columns = Vec::with_capacity(left_schema.columns.len());
                for (left_column, right_column) in
                    left_schema.columns.iter().zip(right_schema.columns.iter())
                {
                    columns.push(BoundColumn {
                        slot_id: context.allocate_slot_id(),
                        name: left_column.name.clone(),
                        table_alias: None,
                        data_type: left_column
                            .data_type
                            .clone()
                            .or_else(|| right_column.data_type.clone()),
                        nullable: left_column.nullable || right_column.nullable,
                        origin: ColumnOrigin::Derived,
                    });
                }

                Ok(RelExpr::SetOperation(crate::algebra::expr::SetOpNode {
                    left: Box::new(left_expr),
                    right: Box::new(right_expr),
                    op: set_op,
                    all,
                    schema: OutputSchema {
                        relation_id: context.allocate_relation_id(),
                        columns,
                    },
                }))
            },
            _ => Err(Diagnostic::todo(
                Phase::Algebraize,
                "set operation and values planning",
            )),
        }
    }

    fn build_select(
        &self,
        select: &Select,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &mut BuildContext,
    ) -> Result<RelExpr, Diagnostic> {
        if select.into.is_some()
            || !select.lateral_views.is_empty()
            || select.prewhere.is_some()
            || !select.cluster_by.is_empty()
            || !select.distribute_by.is_empty()
            || !select.sort_by.is_empty()
            || !select.named_window.is_empty()
            || select.qualify.is_some()
            || select.connect_by.is_some()
            || select.top.is_some()
            || select.value_table_mode.is_some()
        {
            return Err(Diagnostic::todo(
                Phase::Algebraize,
                "advanced SELECT clauses",
            ));
        }

        let group_by_exprs = self.group_by_expressions(&select.group_by)?;

        context.relation_scopes.clear();
        let mut input_expr = self.build_from(select, catalog, functions, context)?;
        if let Some(selection) = &select.selection {
            let (condition, _) = self.bind_expr(selection, catalog, functions, context)?;
            let schema = output_schema_of(&input_expr)?;
            input_expr = RelExpr::Selection(SelectionNode {
                input: Box::new(input_expr),
                condition,
                schema,
            });
        }

        for group_expr in group_by_exprs {
            let (_, has_aggregate) = self.bind_expr(group_expr, catalog, functions, context)?;
            if has_aggregate {
                return Err(Diagnostic::new(
                    "A3016",
                    Phase::Algebraize,
                    "aggregate expression is not allowed in GROUP BY",
                ));
            }
        }

        let group_by_count = group_by_exprs.len();
        let mut has_aggregate = false;
        if let Some(having_expr) = &select.having {
            let (condition, having_has_aggregate) =
                self.bind_expr(having_expr, catalog, functions, context)?;
            has_aggregate |= having_has_aggregate;
            let schema = output_schema_of(&input_expr)?;
            input_expr = RelExpr::Selection(SelectionNode {
                input: Box::new(input_expr),
                condition,
                schema,
            });
        }

        let input_schema = output_schema_of(&input_expr)?;
        let mut projected_columns = Vec::new();
        let mut projection_schema_columns = Vec::new();
        let mut has_non_aggregate_projection = false;

        for select_item in &select.projection {
            match select_item {
                SelectItem::Wildcard(_) => {
                    has_non_aggregate_projection = true;
                    for column in &input_schema.columns {
                        projected_columns.push(ProjectionColumn {
                            expr: BoundScalarExpr::SlotRef(column.slot_id),
                            alias: Some(column.name.clone()),
                            visibility: Visibility::Visible,
                        });
                        projection_schema_columns.push(BoundColumn {
                            slot_id: context.allocate_slot_id(),
                            name: column.name.clone(),
                            table_alias: None,
                            data_type: None,
                            nullable: true,
                            origin: column.origin.clone(),
                        });
                    }
                },
                SelectItem::QualifiedWildcard(qualifier, _) => {
                    has_non_aggregate_projection = true;
                    let qualifier_name = normalize_object_name(qualifier, self.dialect);
                    let Some(scope) = context.relation_scopes.iter().find(|scope| {
                        scope
                            .visible_names
                            .iter()
                            .any(|name| name == &qualifier_name)
                    }) else {
                        return Err(Diagnostic::new(
                            "A3002",
                            Phase::Algebraize,
                            format!("unknown qualified wildcard target: {qualifier_name}"),
                        ));
                    };
                    let scoped_columns = scope.schema.columns.clone();

                    for column in &scoped_columns {
                        projected_columns.push(ProjectionColumn {
                            expr: BoundScalarExpr::SlotRef(column.slot_id),
                            alias: Some(column.name.clone()),
                            visibility: Visibility::Visible,
                        });
                        projection_schema_columns.push(BoundColumn {
                            slot_id: context.allocate_slot_id(),
                            name: column.name.clone(),
                            table_alias: None,
                            data_type: None,
                            nullable: true,
                            origin: column.origin.clone(),
                        });
                    }
                },
                SelectItem::ExprWithAlias { expr, alias } => {
                    self.validate_alias_ident(alias)?;
                    let (bound_expr, expr_has_aggregate) =
                        self.bind_expr(expr, catalog, functions, context)?;
                    has_aggregate |= expr_has_aggregate;
                    if !expr_has_aggregate {
                        has_non_aggregate_projection = true;
                    }
                    let output_name = alias.value.clone();
                    projected_columns.push(ProjectionColumn {
                        expr: bound_expr,
                        alias: Some(output_name.clone()),
                        visibility: Visibility::Visible,
                    });
                    projection_schema_columns.push(BoundColumn {
                        slot_id: context.allocate_slot_id(),
                        name: output_name,
                        table_alias: None,
                        data_type: None,
                        nullable: true,
                        origin: ColumnOrigin::Derived,
                    });
                },
                SelectItem::UnnamedExpr(expr) => {
                    let (bound_expr, expr_has_aggregate) =
                        self.bind_expr(expr, catalog, functions, context)?;
                    has_aggregate |= expr_has_aggregate;
                    if !expr_has_aggregate {
                        has_non_aggregate_projection = true;
                    }
                    let output_name = self.derive_output_name(expr)?;
                    projected_columns.push(ProjectionColumn {
                        expr: bound_expr,
                        alias: Some(output_name.clone()),
                        visibility: Visibility::Visible,
                    });
                    projection_schema_columns.push(BoundColumn {
                        slot_id: context.allocate_slot_id(),
                        name: output_name,
                        table_alias: None,
                        data_type: None,
                        nullable: true,
                        origin: ColumnOrigin::Derived,
                    });
                },
            }
        }

        if group_by_count == 0 && has_aggregate && has_non_aggregate_projection {
            return Err(Diagnostic::new(
                "A3017",
                Phase::Algebraize,
                "non-aggregated projection is not allowed when GROUP BY is absent",
            ));
        }

        let mut relational_expr = RelExpr::Projection(ProjectionNode {
            input: Box::new(input_expr),
            columns: projected_columns,
            schema: OutputSchema {
                relation_id: context.allocate_relation_id(),
                columns: projection_schema_columns,
            },
            is_aggregate: has_aggregate,
            group_by_count,
        });

        if select.distinct.is_some() {
            let schema = output_schema_of(&relational_expr)?;
            relational_expr = RelExpr::Distinct(crate::algebra::expr::DistinctNode {
                input: Box::new(relational_expr),
                schema,
            });
        }

        Ok(relational_expr)
    }

    fn group_by_expressions<'a>(
        &self,
        group_by: &'a GroupByExpr,
    ) -> Result<&'a [Expr], Diagnostic> {
        match group_by {
            GroupByExpr::Expressions(expressions, _) => Ok(expressions.as_slice()),
            _ => Err(Diagnostic::todo(
                Phase::Algebraize,
                "GROUP BY ALL planning and binding",
            )),
        }
    }

    fn apply_top_level_limit_offset(
        &self,
        input_expr: RelExpr,
        query: &sqlparser::ast::Query,
    ) -> Result<RelExpr, Diagnostic> {
        let limit = query
            .limit
            .as_ref()
            .map(|expr| self.parse_non_negative_integer_expr(expr, "LIMIT"))
            .transpose()?;
        let offset = query
            .offset
            .as_ref()
            .map(|offset| self.parse_non_negative_integer_expr(&offset.value, "OFFSET"))
            .transpose()?;

        if limit.is_none() && offset.is_none() {
            return Ok(input_expr);
        }

        let schema = output_schema_of(&input_expr)?;
        Ok(RelExpr::Limit(LimitNode {
            input: Box::new(input_expr),
            limit,
            offset,
            schema,
        }))
    }

    fn parse_non_negative_integer_expr(
        &self,
        expr: &Expr,
        clause_name: &str,
    ) -> Result<u64, Diagnostic> {
        match expr {
            Expr::Value(Value::Number(number, _)) => number.parse::<u64>().map_err(|error| {
                Diagnostic::new(
                    "A3018",
                    Phase::Algebraize,
                    format!("invalid {clause_name} value '{number}': {error}"),
                )
            }),
            Expr::UnaryOp {
                op: UnaryOperator::Plus,
                expr,
            } => self.parse_non_negative_integer_expr(expr, clause_name),
            _ => {
                let message = if clause_name == "LIMIT" {
                    "non-literal LIMIT planning"
                } else {
                    "non-literal OFFSET planning"
                };
                Err(Diagnostic::todo(Phase::Algebraize, message))
            },
        }
    }

    fn build_from(
        &self,
        select: &Select,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &mut BuildContext,
    ) -> Result<RelExpr, Diagnostic> {
        if select.from.is_empty() {
            let schema = OutputSchema {
                relation_id: context.allocate_relation_id(),
                columns: Vec::new(),
            };
            context.relation_scopes = vec![RelationScope {
                visible_names: vec![],
                schema: schema.clone(),
            }];
            return Ok(RelExpr::Values(ValuesNode { schema }));
        }

        if select.from.len() != 1 {
            return Err(Diagnostic::todo(
                Phase::Algebraize,
                "multi-table FROM planning",
            ));
        }

        let from_item = &select.from[0];
        let (mut relation_expr, left_scope) =
            self.build_table_factor(&from_item.relation, catalog, functions, context)?;
        let mut scopes = vec![left_scope];
        context.relation_scopes = scopes.clone();

        for join in &from_item.joins {
            relation_expr = self.build_join(
                relation_expr,
                &mut scopes,
                join,
                catalog,
                functions,
                context,
            )?;
        }

        context.relation_scopes = scopes;
        Ok(relation_expr)
    }

    fn build_table_factor(
        &self,
        relation: &TableFactor,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &mut BuildContext,
    ) -> Result<(RelExpr, RelationScope), Diagnostic> {
        match relation {
            TableFactor::Table { name, alias, .. } => {
                if let Some(alias) = alias {
                    self.validate_alias_ident(&alias.name)?;
                }
                let normalized_table_name = normalize_object_name(name, self.dialect);
                if let Some(cte_binding) = context.ctes.get(&normalized_table_name) {
                    let scope = RelationScope {
                        visible_names: self
                            .visible_names_for_relation(&normalized_table_name, alias.as_ref()),
                        schema: cte_binding.exposed_schema.clone(),
                    };
                    return Ok((cte_binding.expr.clone(), scope));
                }

                let Some(table) = catalog.table(&normalized_table_name) else {
                    return Err(Diagnostic::new(
                        "A3003",
                        Phase::Algebraize,
                        format!("table not found: {normalized_table_name}"),
                    ));
                };

                let (schema, visible_names) =
                    self.build_table_scope(table, &normalized_table_name, alias.as_ref(), context);
                let scope = RelationScope {
                    visible_names,
                    schema: schema.clone(),
                };
                Ok((
                    RelExpr::Scan(ScanNode {
                        table: normalized_table_name,
                        schema,
                    }),
                    scope,
                ))
            },
            TableFactor::Derived {
                lateral,
                subquery,
                alias,
            } => {
                if *lateral {
                    return Err(Diagnostic::todo(
                        Phase::Algebraize,
                        "LATERAL derived table planning",
                    ));
                }

                let mut subquery_context = BuildContext {
                    relation_scopes: context.relation_scopes.clone(),
                    next_relation_id: context.next_relation_id,
                    next_slot_id: context.next_slot_id,
                    ctes: context.ctes.clone(),
                    literal_assignment_mode: true,
                };
                let subquery_expr =
                    self.build_set_expr(&subquery.body, catalog, functions, &mut subquery_context)?;
                context.next_relation_id = subquery_context.next_relation_id;
                context.next_slot_id = subquery_context.next_slot_id;

                let Some(alias) = alias.as_ref() else {
                    return Err(Diagnostic::todo(
                        Phase::Algebraize,
                        "derived table without alias planning",
                    ));
                };
                self.validate_alias_ident(&alias.name)?;
                let alias_name = normalize_ident(&alias.name, self.dialect);

                let mut schema = output_schema_of(&subquery_expr)?;
                if !alias.columns.is_empty() {
                    if alias.columns.len() != schema.columns.len() {
                        return Err(Diagnostic::new(
                            "A3013",
                            Phase::Algebraize,
                            format!(
                                "derived table alias column count mismatch: expected {}, got {}",
                                schema.columns.len(),
                                alias.columns.len()
                            ),
                        ));
                    }

                    for (column, alias_column) in
                        schema.columns.iter_mut().zip(alias.columns.iter())
                    {
                        self.validate_alias_ident(&alias_column.name)?;
                        column.name = normalize_ident(&alias_column.name, self.dialect);
                    }
                }

                let scope = RelationScope {
                    visible_names: vec![alias_name],
                    schema,
                };
                Ok((subquery_expr, scope))
            },
            _ => Err(Diagnostic::todo(
                Phase::Algebraize,
                "derived table and function table factors",
            )),
        }
    }

    fn build_join(
        &self,
        left_expr: RelExpr,
        scopes: &mut Vec<RelationScope>,
        join: &Join,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &mut BuildContext,
    ) -> Result<RelExpr, Diagnostic> {
        if join.global {
            return Err(Diagnostic::todo(Phase::Algebraize, "GLOBAL JOIN planning"));
        }

        let (right_expr, right_scope) =
            self.build_table_factor(&join.relation, catalog, functions, context)?;
        let (kind, on_expr) = self.join_kind_and_condition(&join.join_operator)?;
        let using_columns = self.extract_join_using_columns(&join.join_operator);
        let using_set = using_columns.iter().cloned().collect::<HashSet<String>>();

        let mut visible_right_scope = right_scope.clone();
        if !using_set.is_empty() {
            visible_right_scope
                .schema
                .columns
                .retain(|column| !using_set.contains(&column.name));
        }

        let mut join_scopes = scopes.clone();
        join_scopes.push(right_scope.clone());
        context.relation_scopes = join_scopes;

        let left_schema = output_schema_of(&left_expr)?;
        let right_schema = output_schema_of(&right_expr)?;
        let mut effective_kind = kind.clone();
        let mut bound_condition = None;
        if let Some(on_expr) = on_expr {
            let (condition, _) = self.bind_expr(on_expr, catalog, functions, context)?;
            if self.outer_join_is_effectively_inner(
                kind,
                &condition,
                &left_schema,
                &right_schema,
                catalog,
            ) {
                effective_kind = crate::algebra::expr::JoinKind::Inner;
            }
            bound_condition = Some(condition);
        } else if !using_columns.is_empty() {
            let condition =
                self.build_join_using_condition(&using_columns, &left_schema, &right_schema)?;
            if self.outer_join_is_effectively_inner(
                kind,
                &condition,
                &left_schema,
                &right_schema,
                catalog,
            ) {
                effective_kind = crate::algebra::expr::JoinKind::Inner;
            }
            bound_condition = Some(condition);
        }

        let mut left_columns = left_schema.columns.clone();
        let mut right_columns = right_schema.columns.clone();
        if !using_set.is_empty() {
            right_columns.retain(|column| !using_set.contains(&column.name));
        }
        match effective_kind {
            crate::algebra::expr::JoinKind::Left => {
                for column in &mut right_columns {
                    column.nullable = true;
                }
            },
            crate::algebra::expr::JoinKind::Right => {
                for column in &mut left_columns {
                    column.nullable = true;
                }
            },
            crate::algebra::expr::JoinKind::Full => {
                for column in &mut left_columns {
                    column.nullable = true;
                }
                for column in &mut right_columns {
                    column.nullable = true;
                }
            },
            crate::algebra::expr::JoinKind::Inner | crate::algebra::expr::JoinKind::Cross => {},
        }

        let mut columns = left_columns;
        columns.extend(right_columns);
        let join_schema = OutputSchema {
            relation_id: context.allocate_relation_id(),
            columns,
        };

        let mut join_expr = RelExpr::Join(crate::algebra::expr::JoinNode {
            left: Box::new(left_expr),
            right: Box::new(right_expr),
            kind: effective_kind,
            schema: join_schema.clone(),
        });

        if let Some(condition) = bound_condition {
            join_expr = RelExpr::Selection(SelectionNode {
                input: Box::new(join_expr),
                condition,
                schema: join_schema.clone(),
            });
        }

        scopes.push(visible_right_scope);
        context.relation_scopes = scopes.clone();
        Ok(join_expr)
    }

    fn join_kind_and_condition<'a>(
        &self,
        operator: &'a JoinOperator,
    ) -> Result<(crate::algebra::expr::JoinKind, Option<&'a Expr>), Diagnostic> {
        match operator {
            JoinOperator::Inner(constraint) => {
                self.join_constraint_with_kind(crate::algebra::expr::JoinKind::Inner, constraint)
            },
            JoinOperator::LeftOuter(constraint) => {
                self.join_constraint_with_kind(crate::algebra::expr::JoinKind::Left, constraint)
            },
            JoinOperator::RightOuter(constraint) => {
                self.join_constraint_with_kind(crate::algebra::expr::JoinKind::Right, constraint)
            },
            JoinOperator::FullOuter(constraint) => {
                if matches!(self.dialect, Dialect::MySQL) {
                    return Err(Diagnostic::todo(
                        Phase::Algebraize,
                        "FULL JOIN planning for mysql",
                    ));
                }
                self.join_constraint_with_kind(crate::algebra::expr::JoinKind::Full, constraint)
            },
            JoinOperator::CrossJoin => Ok((crate::algebra::expr::JoinKind::Cross, None)),
            _ => Err(Diagnostic::todo(
                Phase::Algebraize,
                "this JOIN operator planning",
            )),
        }
    }

    fn join_constraint_with_kind<'a>(
        &self,
        kind: crate::algebra::expr::JoinKind,
        constraint: &'a JoinConstraint,
    ) -> Result<(crate::algebra::expr::JoinKind, Option<&'a Expr>), Diagnostic> {
        match constraint {
            JoinConstraint::On(expr) => Ok((kind, Some(expr))),
            JoinConstraint::None => Ok((kind, None)),
            JoinConstraint::Using(_) => Ok((kind, None)),
            JoinConstraint::Natural => Err(Diagnostic::todo(
                Phase::Algebraize,
                "JOIN USING/NATURAL planning",
            )),
        }
    }

    fn extract_join_using_columns(&self, operator: &JoinOperator) -> Vec<String> {
        let constraint = match operator {
            JoinOperator::Inner(constraint)
            | JoinOperator::LeftOuter(constraint)
            | JoinOperator::RightOuter(constraint)
            | JoinOperator::FullOuter(constraint) => constraint,
            _ => return Vec::new(),
        };

        let JoinConstraint::Using(columns) = constraint else {
            return Vec::new();
        };

        columns
            .iter()
            .map(|column| normalize_object_name(column, self.dialect))
            .collect()
    }

    fn build_join_using_condition(
        &self,
        using_columns: &[String],
        left_schema: &OutputSchema,
        right_schema: &OutputSchema,
    ) -> Result<BoundScalarExpr, Diagnostic> {
        let mut condition = None;
        for column_name in using_columns {
            let left_slot = self.resolve_join_using_slot(left_schema, column_name)?;
            let right_slot = self.resolve_join_using_slot(right_schema, column_name)?;
            let equality = BoundScalarExpr::BinaryOp {
                left: Box::new(BoundScalarExpr::SlotRef(left_slot)),
                op: BoundBinaryOp::Eq,
                right: Box::new(BoundScalarExpr::SlotRef(right_slot)),
            };

            condition = Some(match condition {
                Some(existing) => BoundScalarExpr::BinaryOp {
                    left: Box::new(existing),
                    op: BoundBinaryOp::And,
                    right: Box::new(equality),
                },
                None => equality,
            });
        }

        condition.ok_or_else(|| {
            Diagnostic::new(
                "A3027",
                Phase::Algebraize,
                "JOIN USING requires at least one shared column",
            )
        })
    }

    fn resolve_join_using_slot(
        &self,
        schema: &OutputSchema,
        column_name: &str,
    ) -> Result<u32, Diagnostic> {
        let mut slots = schema
            .columns
            .iter()
            .filter(|column| column.name == column_name)
            .map(|column| column.slot_id);

        let Some(slot_id) = slots.next() else {
            return Err(Diagnostic::new(
                "A3008",
                Phase::Algebraize,
                format!("column not found: {column_name}"),
            ));
        };
        if slots.next().is_some() {
            return Err(Diagnostic::new(
                "A3009",
                Phase::Algebraize,
                format!("ambiguous column reference: {column_name}"),
            ));
        }
        Ok(slot_id)
    }

    fn outer_join_is_effectively_inner(
        &self,
        kind: crate::algebra::expr::JoinKind,
        condition: &BoundScalarExpr,
        left_schema: &OutputSchema,
        right_schema: &OutputSchema,
        catalog: &Catalog,
    ) -> bool {
        let (preserved_columns, other_columns) = match kind {
            crate::algebra::expr::JoinKind::Left => (&left_schema.columns, &right_schema.columns),
            crate::algebra::expr::JoinKind::Right => (&right_schema.columns, &left_schema.columns),
            crate::algebra::expr::JoinKind::Inner
            | crate::algebra::expr::JoinKind::Cross
            | crate::algebra::expr::JoinKind::Full => return false,
        };

        let mut slot_pairs = Vec::new();
        collect_equality_slot_pairs(condition, &mut slot_pairs);
        slot_pairs.into_iter().any(|(left_slot, right_slot)| {
            self.fk_slot_pair_guarantees_match(
                left_slot,
                right_slot,
                preserved_columns,
                other_columns,
                catalog,
            ) || self.fk_slot_pair_guarantees_match(
                right_slot,
                left_slot,
                preserved_columns,
                other_columns,
                catalog,
            )
        })
    }

    fn fk_slot_pair_guarantees_match(
        &self,
        preserved_slot: u32,
        other_slot: u32,
        preserved_columns: &[BoundColumn],
        other_columns: &[BoundColumn],
        catalog: &Catalog,
    ) -> bool {
        let Some(preserved_column) = preserved_columns
            .iter()
            .find(|column| column.slot_id == preserved_slot)
        else {
            return false;
        };
        let Some(other_column) = other_columns
            .iter()
            .find(|column| column.slot_id == other_slot)
        else {
            return false;
        };

        if preserved_column.nullable {
            return false;
        }

        let ColumnOrigin::Base {
            table: preserved_table,
            column: preserved_column_name,
        } = &preserved_column.origin
        else {
            return false;
        };
        let ColumnOrigin::Base {
            table: other_table,
            column: other_column_name,
        } = &other_column.origin
        else {
            return false;
        };

        let Some(table_schema) = catalog.table(preserved_table) else {
            return false;
        };

        table_schema.foreign_keys.iter().any(|foreign_key| {
            foreign_key.columns.len() == 1
                && foreign_key.ref_columns.len() == 1
                && foreign_key.columns[0] == *preserved_column_name
                && foreign_key.ref_table == *other_table
                && foreign_key.ref_columns[0] == *other_column_name
        })
    }

    fn visible_names_for_relation(
        &self,
        normalized_table_name: &str,
        alias: Option<&sqlparser::ast::TableAlias>,
    ) -> Vec<String> {
        if let Some(alias) = alias {
            return vec![normalize_ident(&alias.name, self.dialect)];
        }

        let mut visible_names = Vec::new();
        visible_names.push(normalized_table_name.to_string());
        if let Some(last_segment) = normalized_table_name.split('.').next_back() {
            if !visible_names.iter().any(|name| name == last_segment) {
                visible_names.push(last_segment.to_string());
            }
        }
        visible_names
    }

    fn build_table_scope(
        &self,
        table: &TableSchema,
        normalized_table_name: &str,
        alias: Option<&sqlparser::ast::TableAlias>,
        context: &mut BuildContext,
    ) -> (OutputSchema, Vec<String>) {
        let mut columns = Vec::with_capacity(table.columns.len());
        for column in &table.columns {
            let slot_id = context.allocate_slot_id();
            columns.push(BoundColumn {
                slot_id,
                name: column.name.clone(),
                table_alias: alias
                    .as_ref()
                    .map(|table_alias| normalize_ident(&table_alias.name, self.dialect))
                    .or_else(|| {
                        normalized_table_name
                            .split('.')
                            .next_back()
                            .map(|name| name.to_string())
                    }),
                data_type: Some(column.data_type.clone()),
                nullable: column.nullable,
                origin: ColumnOrigin::Base {
                    table: normalized_table_name.to_string(),
                    column: column.name.clone(),
                },
            });
        }

        let visible_names = self.visible_names_for_relation(normalized_table_name, alias);

        (
            OutputSchema {
                relation_id: context.allocate_relation_id(),
                columns,
            },
            visible_names,
        )
    }

    fn bind_expr(
        &self,
        expr: &Expr,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &BuildContext,
    ) -> Result<(BoundScalarExpr, bool), Diagnostic> {
        match expr {
            Expr::Identifier(ident) => {
                let normalized = normalize_ident(ident, self.dialect);
                let column = self.resolve_unqualified_column(&normalized, context)?;
                Ok((BoundScalarExpr::SlotRef(column.slot_id), false))
            },
            Expr::CompoundIdentifier(idents) => {
                if idents.is_empty() {
                    return Err(Diagnostic::new(
                        "A3004",
                        Phase::Algebraize,
                        "empty compound identifier",
                    ));
                }
                let column_name = normalize_ident(idents.last().expect("not empty"), self.dialect);
                let qualifier = idents[..idents.len() - 1]
                    .iter()
                    .map(|ident| normalize_ident(ident, self.dialect))
                    .collect::<Vec<_>>()
                    .join(".");
                let column = self.resolve_qualified_column(&qualifier, &column_name, context)?;
                Ok((BoundScalarExpr::SlotRef(column.slot_id), false))
            },
            Expr::Value(value) => Ok((
                BoundScalarExpr::Literal(
                    self.bind_literal(value, context.literal_assignment_mode)?,
                ),
                false,
            )),
            Expr::Nested(inner) => self.bind_expr(inner, catalog, functions, context),
            Expr::UnaryOp { op, expr } => {
                let (inner, has_aggregate) = self.bind_expr(expr, catalog, functions, context)?;
                let op = match op {
                    UnaryOperator::Plus => BoundUnaryOp::Pos,
                    UnaryOperator::Minus => BoundUnaryOp::Neg,
                    UnaryOperator::Not => BoundUnaryOp::Not,
                    _ => {
                        return Err(Diagnostic::todo(
                            Phase::Algebraize,
                            "this unary operator binding",
                        ));
                    },
                };
                Ok((
                    BoundScalarExpr::UnaryOp {
                        op,
                        expr: Box::new(inner),
                    },
                    has_aggregate,
                ))
            },
            Expr::BinaryOp { left, op, right } => {
                let (left_expr, left_has_aggregate) =
                    self.bind_expr(left, catalog, functions, context)?;
                let (right_expr, right_has_aggregate) =
                    self.bind_expr(right, catalog, functions, context)?;
                let op = map_binary_operator(op)?;
                Ok((
                    BoundScalarExpr::BinaryOp {
                        left: Box::new(left_expr),
                        op,
                        right: Box::new(right_expr),
                    },
                    left_has_aggregate || right_has_aggregate,
                ))
            },
            Expr::Cast {
                expr, data_type, ..
            } => {
                let (inner, has_aggregate) = self.bind_expr(expr, catalog, functions, context)?;
                let target_type = ddl_type_map::map_sql_data_type(self.dialect, data_type);
                Ok((
                    BoundScalarExpr::Cast {
                        expr: Box::new(inner),
                        target_type,
                    },
                    has_aggregate,
                ))
            },
            Expr::Ceil { expr, field } => {
                self.bind_ceil_or_floor("ceil", expr, field, catalog, functions, context)
            },
            Expr::Floor { expr, field } => {
                self.bind_ceil_or_floor("floor", expr, field, catalog, functions, context)
            },
            Expr::Trim {
                expr,
                trim_where,
                trim_what,
                trim_characters,
            } => {
                if trim_where.is_some() || trim_what.is_some() || trim_characters.is_some() {
                    return Err(Diagnostic::todo(
                        Phase::Algebraize,
                        "TRIM modifiers binding",
                    ));
                }
                let (bound_expr, has_aggregate) =
                    self.bind_expr(expr, catalog, functions, context)?;
                let bound_args = vec![bound_expr];
                self.validate_function_arity("trim", bound_args.len(), functions)?;
                self.validate_function_argument_types("trim", &bound_args, context)?;
                Ok((
                    BoundScalarExpr::Function {
                        name: "trim".to_string(),
                        args: bound_args,
                    },
                    has_aggregate,
                ))
            },
            Expr::Function(function) => self.bind_function(function, catalog, functions, context),
            Expr::IsNull(inner) => {
                let (bound, has_aggregate) = self.bind_expr(inner, catalog, functions, context)?;
                Ok((
                    BoundScalarExpr::IsNull {
                        expr: Box::new(bound),
                        negated: false,
                    },
                    has_aggregate,
                ))
            },
            Expr::IsNotNull(inner) => {
                let (bound, has_aggregate) = self.bind_expr(inner, catalog, functions, context)?;
                Ok((
                    BoundScalarExpr::IsNull {
                        expr: Box::new(bound),
                        negated: true,
                    },
                    has_aggregate,
                ))
            },
            Expr::Case {
                operand,
                conditions,
                results,
                else_result,
            } => {
                let mut has_aggregate = false;
                let mut when_clauses = Vec::new();

                for (condition, result) in conditions.iter().zip(results.iter()) {
                    let (bound_condition, condition_has_aggregate) =
                        self.bind_expr(condition, catalog, functions, context)?;
                    let (bound_result, result_has_aggregate) =
                        self.bind_expr(result, catalog, functions, context)?;
                    has_aggregate |= condition_has_aggregate || result_has_aggregate;
                    when_clauses.push((bound_condition, bound_result));
                }

                let bound_operand = if let Some(operand) = operand {
                    let (bound, operand_has_aggregate) =
                        self.bind_expr(operand, catalog, functions, context)?;
                    has_aggregate |= operand_has_aggregate;
                    Some(Box::new(bound))
                } else {
                    None
                };

                let bound_else = if let Some(else_expr) = else_result {
                    let (bound, else_has_aggregate) =
                        self.bind_expr(else_expr, catalog, functions, context)?;
                    has_aggregate |= else_has_aggregate;
                    Some(Box::new(bound))
                } else {
                    None
                };

                Ok((
                    BoundScalarExpr::Case {
                        operand: bound_operand,
                        when_clauses,
                        else_expr: bound_else,
                    },
                    has_aggregate,
                ))
            },
            Expr::InList {
                expr,
                list,
                negated,
            } => {
                let (bound_expr, mut has_aggregate) =
                    self.bind_expr(expr, catalog, functions, context)?;
                let mut bound_list = Vec::with_capacity(list.len());
                for item in list {
                    let (bound_item, item_has_aggregate) =
                        self.bind_expr(item, catalog, functions, context)?;
                    has_aggregate |= item_has_aggregate;
                    bound_list.push(bound_item);
                }
                Ok((
                    BoundScalarExpr::InList {
                        expr: Box::new(bound_expr),
                        list: bound_list,
                        negated: *negated,
                    },
                    has_aggregate,
                ))
            },
            Expr::InSubquery { expr, negated, .. } => {
                let (bound_expr, has_aggregate) =
                    self.bind_expr(expr, catalog, functions, context)?;
                Ok((
                    BoundScalarExpr::InSubquery {
                        expr: Box::new(bound_expr),
                        negated: *negated,
                    },
                    has_aggregate,
                ))
            },
            Expr::Exists { negated, .. } => {
                Ok((BoundScalarExpr::Exists { negated: *negated }, false))
            },
            Expr::Subquery(query) => {
                let (data_type, nullable) = self.infer_scalar_subquery_result(query, catalog);
                Ok((
                    BoundScalarExpr::ScalarSubquery {
                        data_type,
                        nullable,
                    },
                    false,
                ))
            },
            _ => Err(Diagnostic::todo(
                Phase::Algebraize,
                "this scalar expression binding",
            )),
        }
    }

    fn bind_function(
        &self,
        function: &Function,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &BuildContext,
    ) -> Result<(BoundScalarExpr, bool), Diagnostic> {
        let function_name = normalize_object_name(&function.name, self.dialect);
        let function_name_lower = function_name.to_ascii_lowercase();

        let mut bound_args = Vec::new();
        let mut has_aggregate_in_args = false;

        match &function.args {
            FunctionArguments::None => {},
            FunctionArguments::Subquery(_) => {
                return Err(Diagnostic::todo(
                    Phase::Algebraize,
                    "function subquery argument binding",
                ));
            },
            FunctionArguments::List(argument_list) => {
                for arg in &argument_list.args {
                    let (bound_arg, arg_has_aggregate) =
                        self.bind_function_arg(arg, catalog, functions, context)?;
                    bound_args.push(bound_arg);
                    has_aggregate_in_args |= arg_has_aggregate;
                }
            },
        }

        self.validate_function_arity(&function_name_lower, bound_args.len(), functions)?;
        self.validate_function_argument_types(&function_name_lower, &bound_args, context)?;

        let distinct = matches!(
            &function.args,
            FunctionArguments::List(list)
                if list.duplicate_treatment.is_some_and(|value| matches!(value, sqlparser::ast::DuplicateTreatment::Distinct))
        );

        if function.over.is_some() {
            let (partition_by, order_by) = match &function.over {
                Some(WindowType::WindowSpec(spec)) => {
                    let mut partition_by = Vec::new();
                    for expr in &spec.partition_by {
                        let (bound_expr, _) = self.bind_expr(expr, catalog, functions, context)?;
                        partition_by.push(bound_expr);
                    }
                    (partition_by, Vec::new())
                },
                Some(WindowType::NamedWindow(_)) => {
                    return Err(Diagnostic::todo(
                        Phase::Algebraize,
                        "named window reference binding",
                    ));
                },
                None => (Vec::new(), Vec::new()),
            };

            return Ok((
                BoundScalarExpr::WindowCall {
                    name: function_name,
                    args: bound_args,
                    partition_by,
                    order_by,
                },
                has_aggregate_in_args,
            ));
        }

        let is_aggregate = functions
            .resolve(&function_name_lower)
            .is_some_and(|signature| signature.category == FunctionCategory::Aggregate);

        if is_aggregate {
            return Ok((
                BoundScalarExpr::AggregateCall {
                    name: function_name,
                    args: bound_args,
                    distinct,
                },
                true,
            ));
        }

        Ok((
            BoundScalarExpr::Function {
                name: function_name,
                args: bound_args,
            },
            has_aggregate_in_args,
        ))
    }

    fn bind_function_arg(
        &self,
        arg: &FunctionArg,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &BuildContext,
    ) -> Result<(BoundScalarExpr, bool), Diagnostic> {
        let arg_expr = match arg {
            FunctionArg::Named { arg, .. } => arg,
            FunctionArg::ExprNamed { arg, .. } => arg,
            FunctionArg::Unnamed(arg) => arg,
        };

        match arg_expr {
            FunctionArgExpr::Expr(expr) => self.bind_expr(expr, catalog, functions, context),
            FunctionArgExpr::Wildcard => Ok((BoundScalarExpr::Placeholder("*".to_string()), false)),
            FunctionArgExpr::QualifiedWildcard(prefix) => Ok((
                BoundScalarExpr::Placeholder(format!(
                    "{}.*",
                    normalize_object_name(prefix, self.dialect)
                )),
                false,
            )),
        }
    }

    fn validate_function_arity(
        &self,
        function_name_lower: &str,
        arity: usize,
        functions: &FunctionRegistry,
    ) -> Result<(), Diagnostic> {
        let Some(signature) = functions.resolve(function_name_lower) else {
            return Ok(());
        };

        if arity < signature.min_arity {
            return Err(Diagnostic::new(
                "A3020",
                Phase::Algebraize,
                format!(
                    "function '{}' expects at least {} argument(s), got {}",
                    function_name_lower, signature.min_arity, arity
                ),
            ));
        }
        if let Some(max_arity) = signature.max_arity {
            if arity > max_arity {
                return Err(Diagnostic::new(
                    "A3021",
                    Phase::Algebraize,
                    format!(
                        "function '{}' expects at most {} argument(s), got {}",
                        function_name_lower, max_arity, arity
                    ),
                ));
            }
        }

        Ok(())
    }

    fn validate_function_argument_types(
        &self,
        function_name_lower: &str,
        bound_args: &[BoundScalarExpr],
        context: &BuildContext,
    ) -> Result<(), Diagnostic> {
        if matches!(self.dialect, Dialect::SQLite) {
            if matches!(function_name_lower, "ceil" | "floor") {
                self.require_postgres_numeric_arg(function_name_lower, bound_args, context, 0)?;
            }
            return Ok(());
        }

        if !matches!(self.dialect, Dialect::Postgres) {
            return Ok(());
        }

        match function_name_lower {
            "upper" | "lower" | "trim" | "ltrim" | "rtrim" | "length" | "char_length"
            | "substr" => {
                self.require_postgres_text_arg(function_name_lower, bound_args, context, 0)?;
            },
            "abs" | "ceil" | "floor" | "round" | "sqrt" | "exp" | "ln" | "log10" | "sign"
            | "sum" | "avg" => {
                self.require_postgres_numeric_arg(function_name_lower, bound_args, context, 0)?;
            },
            "power" | "mod" => {
                self.require_postgres_numeric_arg(function_name_lower, bound_args, context, 0)?;
                self.require_postgres_numeric_arg(function_name_lower, bound_args, context, 1)?;
            },
            _ => {},
        }

        Ok(())
    }

    fn bind_ceil_or_floor(
        &self,
        function_name: &str,
        expr: &Expr,
        field: &CeilFloorKind,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &BuildContext,
    ) -> Result<(BoundScalarExpr, bool), Diagnostic> {
        let (bound_expr, has_aggregate) = self.bind_expr(expr, catalog, functions, context)?;
        let bound_args = match field {
            CeilFloorKind::DateTimeField(DateTimeField::NoDateTime) => vec![bound_expr],
            CeilFloorKind::DateTimeField(_) | CeilFloorKind::Scale(_) => {
                return Err(Diagnostic::todo(
                    Phase::Algebraize,
                    "CEIL/FLOOR modifiers binding",
                ));
            },
        };

        self.validate_function_arity(function_name, bound_args.len(), functions)?;
        self.validate_function_argument_types(function_name, &bound_args, context)?;

        Ok((
            BoundScalarExpr::Function {
                name: function_name.to_string(),
                args: bound_args,
            },
            has_aggregate,
        ))
    }

    fn validate_alias_ident(&self, alias: &sqlparser::ast::Ident) -> Result<(), Diagnostic> {
        if !matches!(self.dialect, Dialect::MySQL) {
            return Ok(());
        }
        if alias.quote_style.is_some() {
            return Ok(());
        }

        let alias_lower = alias.value.to_ascii_lowercase();
        if matches!(
            alias_lower.as_str(),
            "select" | "window" | "rank" | "row_number"
        ) {
            return Err(Diagnostic::new(
                "A3024",
                Phase::Algebraize,
                format!("reserved keyword cannot be used as alias: {}", alias.value),
            ));
        }

        Ok(())
    }

    fn require_postgres_text_arg(
        &self,
        function_name_lower: &str,
        bound_args: &[BoundScalarExpr],
        context: &BuildContext,
        index: usize,
    ) -> Result<(), Diagnostic> {
        let Some(arg_type) = self.bound_expr_static_type(bound_args.get(index), context) else {
            return Ok(());
        };
        if arg_type.is_text_like() {
            return Ok(());
        }

        Err(Diagnostic::new(
            "A3022",
            Phase::Algebraize,
            format!(
                "function '{}' expects text argument at position {}",
                function_name_lower,
                index + 1
            ),
        ))
    }

    fn require_postgres_numeric_arg(
        &self,
        function_name_lower: &str,
        bound_args: &[BoundScalarExpr],
        context: &BuildContext,
        index: usize,
    ) -> Result<(), Diagnostic> {
        let Some(arg_type) = self.bound_expr_static_type(bound_args.get(index), context) else {
            return Ok(());
        };
        if arg_type.is_numeric() {
            return Ok(());
        }

        Err(Diagnostic::new(
            "A3023",
            Phase::Algebraize,
            format!(
                "function '{}' expects numeric argument at position {}",
                function_name_lower,
                index + 1
            ),
        ))
    }

    fn bound_expr_static_type(
        &self,
        expr: Option<&BoundScalarExpr>,
        context: &BuildContext,
    ) -> Option<DataType> {
        let expr = expr?;
        match expr {
            BoundScalarExpr::SlotRef(slot_id) => context
                .relation_scopes
                .iter()
                .flat_map(|scope| scope.schema.columns.iter())
                .find(|column| column.slot_id == *slot_id)
                .and_then(|column| column.data_type.clone()),
            BoundScalarExpr::Literal(literal) => self.bound_literal_static_type(literal),
            BoundScalarExpr::Cast { target_type, .. } => Some(target_type.clone()),
            _ => None,
        }
    }

    fn bound_literal_static_type(&self, literal: &BoundLiteral) -> Option<DataType> {
        match literal {
            BoundLiteral::Null => None,
            BoundLiteral::Bool(_) => Some(match self.dialect {
                Dialect::Postgres => DataType::Bool,
                Dialect::MySQL | Dialect::SQLite => DataType::BigInt,
            }),
            BoundLiteral::Int { .. } => Some(match self.dialect {
                Dialect::Postgres => DataType::Int,
                Dialect::MySQL | Dialect::SQLite => DataType::BigInt,
            }),
            BoundLiteral::Float(_) => Some(match self.dialect {
                Dialect::SQLite => DataType::Double,
                Dialect::MySQL | Dialect::Postgres => DataType::Decimal,
            }),
            BoundLiteral::String(_) => Some(match self.dialect {
                Dialect::MySQL => DataType::Varchar,
                Dialect::Postgres | Dialect::SQLite => DataType::Text,
            }),
            BoundLiteral::Placeholder(_) => None,
        }
    }

    fn bind_literal(
        &self,
        value: &Value,
        literal_assignment_mode: bool,
    ) -> Result<BoundLiteral, Diagnostic> {
        let literal = match value {
            Value::Boolean(boolean) => BoundLiteral::Bool(*boolean),
            Value::Null => BoundLiteral::Null,
            Value::SingleQuotedString(value)
            | Value::TripleSingleQuotedString(value)
            | Value::EscapedStringLiteral(value)
            | Value::UnicodeStringLiteral(value)
            | Value::NationalStringLiteral(value)
            | Value::DoubleQuotedString(value)
            | Value::TripleDoubleQuotedString(value)
            | Value::SingleQuotedRawStringLiteral(value)
            | Value::DoubleQuotedRawStringLiteral(value)
            | Value::TripleSingleQuotedRawStringLiteral(value)
            | Value::TripleDoubleQuotedRawStringLiteral(value) => {
                BoundLiteral::String(value.clone())
            },
            Value::Number(number, _) => {
                if number.contains('.') || number.contains('e') || number.contains('E') {
                    let parsed = number.parse::<f64>().map_err(|err| {
                        Diagnostic::new(
                            "A3005",
                            Phase::Algebraize,
                            format!("invalid floating literal '{number}': {err}"),
                        )
                    })?;
                    BoundLiteral::Float(parsed)
                } else {
                    let parsed = number.parse::<i64>().map_err(|err| {
                        Diagnostic::new(
                            "A3006",
                            Phase::Algebraize,
                            format!("invalid integer literal '{number}': {err}"),
                        )
                    })?;
                    BoundLiteral::Int {
                        value: parsed,
                        raw: number.clone(),
                        assignment: literal_assignment_mode,
                    }
                }
            },
            Value::Placeholder(value) => BoundLiteral::Placeholder(value.clone()),
            _ => {
                return Err(Diagnostic::todo(
                    Phase::Algebraize,
                    "this literal kind binding",
                ));
            },
        };
        Ok(literal)
    }

    fn resolve_unqualified_column<'a>(
        &self,
        column_name: &str,
        context: &'a BuildContext,
    ) -> Result<&'a BoundColumn, Diagnostic> {
        let columns = context.current_columns();
        let mut matched = columns
            .into_iter()
            .filter(|column| column.name == column_name);
        let first = matched.next();
        let second = matched.next();

        match (first, second) {
            (Some(column), None) => context
                .relation_scopes
                .iter()
                .flat_map(|scope| scope.schema.columns.iter())
                .find(|candidate| candidate.slot_id == column.slot_id)
                .ok_or_else(|| {
                    Diagnostic::new(
                        "A3007",
                        Phase::Algebraize,
                        format!("failed to resolve column '{column_name}'"),
                    )
                }),
            (None, _) => Err(Diagnostic::new(
                "A3008",
                Phase::Algebraize,
                format!("column not found: {column_name}"),
            )),
            (Some(_), Some(_)) => Err(Diagnostic::new(
                "A3009",
                Phase::Algebraize,
                format!("ambiguous column reference: {column_name}"),
            )),
        }
    }

    fn resolve_qualified_column<'a>(
        &self,
        qualifier: &str,
        column_name: &str,
        context: &'a BuildContext,
    ) -> Result<&'a BoundColumn, Diagnostic> {
        let Some(scope) = context.relation_scopes.iter().find(|scope| {
            scope
                .visible_names
                .iter()
                .any(|visible_name| visible_name == qualifier)
        }) else {
            return Err(Diagnostic::new(
                "A3010",
                Phase::Algebraize,
                format!("unknown relation reference: {qualifier}"),
            ));
        };

        scope
            .schema
            .columns
            .iter()
            .find(|column| column.name == column_name)
            .ok_or_else(|| {
                Diagnostic::new(
                    "A3011",
                    Phase::Algebraize,
                    format!("column not found: {qualifier}.{column_name}"),
                )
            })
    }

    fn derive_output_name(&self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Identifier(ident) => Ok(normalize_ident(ident, self.dialect)),
            Expr::CompoundIdentifier(idents) => {
                let last = idents.last().ok_or_else(|| {
                    Diagnostic::new(
                        "A3012",
                        Phase::Algebraize,
                        "empty compound identifier in projection",
                    )
                })?;
                Ok(normalize_ident(last, self.dialect))
            },
            Expr::Function(function) => {
                let function_name = normalize_object_name(&function.name, self.dialect);
                match self.dialect {
                    Dialect::Postgres => Ok(function_name.to_ascii_lowercase()),
                    Dialect::MySQL | Dialect::SQLite => Ok(expr.to_string()),
                }
            },
            Expr::Case { .. } => match self.dialect {
                Dialect::Postgres => Ok("case".to_string()),
                Dialect::MySQL | Dialect::SQLite => Ok(expr.to_string()),
            },
            Expr::Value(Value::SingleQuotedString(value)) => match self.dialect {
                Dialect::Postgres => Ok("?column?".to_string()),
                Dialect::MySQL => Ok(value.clone()),
                Dialect::SQLite => Ok(format!("'{value}'")),
            },
            Expr::Value(Value::Boolean(value)) => match self.dialect {
                Dialect::Postgres => Ok("?column?".to_string()),
                Dialect::MySQL | Dialect::SQLite => {
                    if *value {
                        Ok("TRUE".to_string())
                    } else {
                        Ok("FALSE".to_string())
                    }
                },
            },
            Expr::Value(_) => match self.dialect {
                Dialect::Postgres => Ok("?column?".to_string()),
                Dialect::MySQL | Dialect::SQLite => Ok(expr.to_string()),
            },
            _ => match self.dialect {
                Dialect::Postgres => Ok("?column?".to_string()),
                Dialect::MySQL | Dialect::SQLite => Ok(expr.to_string()),
            },
        }
    }

    fn infer_scalar_subquery_result(
        &self,
        query: &sqlparser::ast::Query,
        catalog: &Catalog,
    ) -> (DataType, bool) {
        let SetExpr::Select(select) = &*query.body else {
            return (DataType::Custom("unknown".to_string()), true);
        };
        if select.projection.len() != 1 {
            return (DataType::Custom("unknown".to_string()), true);
        }

        let item_expr = match &select.projection[0] {
            SelectItem::UnnamedExpr(expr) => Some(expr),
            SelectItem::ExprWithAlias { expr, .. } => Some(expr),
            _ => None,
        };
        let Some(item_expr) = item_expr else {
            return (DataType::Custom("unknown".to_string()), true);
        };

        if let Expr::Function(function) = item_expr {
            let function_name = normalize_object_name(&function.name, self.dialect);
            let function_name = function_name.to_ascii_lowercase();
            match function_name.as_str() {
                "count" => {
                    return (DataType::BigInt, false);
                },
                "max" | "min" => {
                    if let Some(arg_expr) = first_function_expr_arg(function) {
                        if let Some((data_type, _)) =
                            self.resolve_scalar_subquery_expr_type(select, arg_expr, catalog)
                        {
                            return (data_type, true);
                        }
                    }
                },
                "sum" => {
                    if let Some(arg_expr) = first_function_expr_arg(function) {
                        if let Some((arg_type, _)) =
                            self.resolve_scalar_subquery_expr_type(select, arg_expr, catalog)
                        {
                            let data_type = match self.dialect {
                                Dialect::Postgres => {
                                    if arg_type.is_integer() {
                                        DataType::BigInt
                                    } else {
                                        arg_type
                                    }
                                },
                                Dialect::MySQL | Dialect::SQLite => {
                                    if arg_type.is_numeric() {
                                        arg_type
                                    } else {
                                        DataType::Double
                                    }
                                },
                            };
                            return (data_type, true);
                        }
                    }
                },
                "avg" => {
                    return (DataType::Decimal, true);
                },
                _ => {},
            }
        }

        if let Some((data_type, _)) =
            self.resolve_scalar_subquery_expr_type(select, item_expr, catalog)
        {
            return (data_type, true);
        }

        (DataType::Custom("unknown".to_string()), true)
    }

    fn resolve_scalar_subquery_expr_type(
        &self,
        select: &Select,
        expr: &Expr,
        catalog: &Catalog,
    ) -> Option<(DataType, bool)> {
        match expr {
            Expr::Identifier(identifier) => {
                self.resolve_subquery_column(select, None, &identifier.value, catalog)
            },
            Expr::CompoundIdentifier(idents) => {
                if idents.len() < 2 {
                    return None;
                }
                let qualifier = idents[..idents.len() - 1]
                    .iter()
                    .map(|ident| normalize_ident(ident, self.dialect))
                    .collect::<Vec<_>>()
                    .join(".");
                let column_name = normalize_ident(idents.last()?, self.dialect);
                self.resolve_subquery_column(select, Some(&qualifier), &column_name, catalog)
            },
            Expr::Value(value) => {
                let bound_literal = self.bind_literal(value, false).ok()?;
                match bound_literal {
                    BoundLiteral::Null => Some((DataType::Custom("null".to_string()), true)),
                    BoundLiteral::Bool(_) => Some((
                        match self.dialect {
                            Dialect::Postgres => DataType::Bool,
                            Dialect::MySQL | Dialect::SQLite => DataType::BigInt,
                        },
                        false,
                    )),
                    BoundLiteral::Int {
                        raw, assignment, ..
                    } => Some((
                        match self.dialect {
                            Dialect::Postgres => DataType::Int,
                            Dialect::MySQL => {
                                if assignment && mysql_integer_literal_should_be_int(&raw) {
                                    DataType::Int
                                } else {
                                    DataType::BigInt
                                }
                            },
                            Dialect::SQLite => DataType::BigInt,
                        },
                        false,
                    )),
                    BoundLiteral::Float(_) => Some((
                        match self.dialect {
                            Dialect::SQLite => DataType::Double,
                            Dialect::MySQL | Dialect::Postgres => DataType::Decimal,
                        },
                        false,
                    )),
                    BoundLiteral::String(_) => Some((
                        match self.dialect {
                            Dialect::MySQL => DataType::Varchar,
                            Dialect::Postgres | Dialect::SQLite => DataType::Text,
                        },
                        false,
                    )),
                    BoundLiteral::Placeholder(_) => {
                        Some((DataType::Custom("unknown".to_string()), false))
                    },
                }
            },
            _ => None,
        }
    }

    fn resolve_subquery_column(
        &self,
        select: &Select,
        qualifier: Option<&str>,
        column_name: &str,
        catalog: &Catalog,
    ) -> Option<(DataType, bool)> {
        if select.from.len() != 1 {
            return None;
        }

        let from_item = &select.from[0];
        if !from_item.joins.is_empty() {
            return None;
        }

        let TableFactor::Table { name, alias, .. } = &from_item.relation else {
            return None;
        };

        let normalized_table_name = normalize_object_name(name, self.dialect);
        let table = catalog.table(&normalized_table_name)?;

        if let Some(qualifier) = qualifier {
            let mut visible_names = vec![normalized_table_name.clone()];
            if let Some(last_segment) = normalized_table_name.split('.').next_back() {
                if !visible_names.iter().any(|name| name == last_segment) {
                    visible_names.push(last_segment.to_string());
                }
            }
            if let Some(alias) = alias {
                visible_names.push(normalize_ident(&alias.name, self.dialect));
            }

            if !visible_names.iter().any(|name| name == qualifier) {
                return None;
            }
        }

        let normalized_column_name = normalize_column_name(column_name, self.dialect);
        let column = table
            .columns
            .iter()
            .find(|column| column.name == normalized_column_name)?;
        Some((column.data_type.clone(), column.nullable))
    }
}

fn first_function_expr_arg(function: &Function) -> Option<&Expr> {
    let FunctionArguments::List(argument_list) = &function.args else {
        return None;
    };
    let first = argument_list.args.first()?;
    let arg_expr = match first {
        FunctionArg::Named { arg, .. } => arg,
        FunctionArg::ExprNamed { arg, .. } => arg,
        FunctionArg::Unnamed(arg) => arg,
    };
    match arg_expr {
        FunctionArgExpr::Expr(expr) => Some(expr),
        FunctionArgExpr::Wildcard | FunctionArgExpr::QualifiedWildcard(_) => None,
    }
}

fn normalize_column_name(name: &str, dialect: Dialect) -> String {
    if matches!(dialect, Dialect::Postgres) {
        name.to_ascii_lowercase()
    } else {
        name.to_string()
    }
}

fn recursive_cte_seed_select(set_expr: &SetExpr) -> Option<&Select> {
    match set_expr {
        SetExpr::Select(select) => Some(select),
        SetExpr::SetOperation { left, .. } => recursive_cte_seed_select(left),
        SetExpr::Query(query) => recursive_cte_seed_select(&query.body),
        _ => None,
    }
}

fn recursive_cte_set_operation_projection_counts(set_expr: &SetExpr) -> Option<(usize, usize)> {
    match set_expr {
        SetExpr::SetOperation { left, right, .. } => Some((
            recursive_cte_projection_count(left)?,
            recursive_cte_projection_count(right)?,
        )),
        SetExpr::Query(query) => recursive_cte_set_operation_projection_counts(&query.body),
        _ => None,
    }
}

fn recursive_cte_projection_count(set_expr: &SetExpr) -> Option<usize> {
    match set_expr {
        SetExpr::Select(select) => Some(select.projection.len()),
        SetExpr::SetOperation { left, .. } => recursive_cte_projection_count(left),
        SetExpr::Query(query) => recursive_cte_projection_count(&query.body),
        _ => None,
    }
}

fn projection_expr(item: &SelectItem) -> Option<&Expr> {
    match item {
        SelectItem::UnnamedExpr(expr) => Some(expr),
        SelectItem::ExprWithAlias { expr, .. } => Some(expr),
        SelectItem::QualifiedWildcard(_, _) | SelectItem::Wildcard(_) => None,
    }
}

fn mysql_integer_literal_should_be_int(raw: &str) -> bool {
    let digit_count = raw.chars().filter(|c| c.is_ascii_digit()).count();
    digit_count <= 8
}

fn map_binary_operator(operator: &BinaryOperator) -> Result<BoundBinaryOp, Diagnostic> {
    let mapped = match operator {
        BinaryOperator::Plus => BoundBinaryOp::Add,
        BinaryOperator::Minus => BoundBinaryOp::Sub,
        BinaryOperator::Multiply => BoundBinaryOp::Mul,
        BinaryOperator::Divide => BoundBinaryOp::Div,
        BinaryOperator::Eq => BoundBinaryOp::Eq,
        BinaryOperator::NotEq => BoundBinaryOp::NotEq,
        BinaryOperator::Lt => BoundBinaryOp::Lt,
        BinaryOperator::LtEq => BoundBinaryOp::Lte,
        BinaryOperator::Gt => BoundBinaryOp::Gt,
        BinaryOperator::GtEq => BoundBinaryOp::Gte,
        BinaryOperator::And => BoundBinaryOp::And,
        BinaryOperator::Or => BoundBinaryOp::Or,
        _ => {
            return Err(Diagnostic::todo(
                Phase::Algebraize,
                "this binary operator binding",
            ));
        },
    };
    Ok(mapped)
}

fn collect_equality_slot_pairs(expr: &BoundScalarExpr, output: &mut Vec<(u32, u32)>) {
    match expr {
        BoundScalarExpr::BinaryOp {
            left,
            op: BoundBinaryOp::And,
            right,
        } => {
            collect_equality_slot_pairs(left, output);
            collect_equality_slot_pairs(right, output);
        },
        BoundScalarExpr::BinaryOp {
            left,
            op: BoundBinaryOp::Eq,
            right,
        } => {
            if let (BoundScalarExpr::SlotRef(left_slot), BoundScalarExpr::SlotRef(right_slot)) =
                (&**left, &**right)
            {
                output.push((*left_slot, *right_slot));
            }
        },
        _ => {},
    }
}

fn output_schema_of(expr: &RelExpr) -> Result<OutputSchema, Diagnostic> {
    match expr {
        RelExpr::Scan(node) => Ok(node.schema.clone()),
        RelExpr::Values(node) => Ok(node.schema.clone()),
        RelExpr::Selection(node) => Ok(node.schema.clone()),
        RelExpr::Projection(node) => Ok(node.schema.clone()),
        RelExpr::Distinct(node) => Ok(node.schema.clone()),
        RelExpr::Sort(node) => Ok(node.schema.clone()),
        RelExpr::Limit(node) => Ok(node.schema.clone()),
        RelExpr::Alias(node) => Ok(node.schema.clone()),
        RelExpr::SetOperation(node) => Ok(node.schema.clone()),
        _ => Err(Diagnostic::todo(
            Phase::Algebraize,
            "schema extraction for this relation node",
        )),
    }
}
