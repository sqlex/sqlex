//! Main query analyzer.

use std::collections::HashMap;

use sqlex_parser::{
    Expr, Query, Select, SelectItem, SetExpr, Statement, TableWithJoins, parse_one, sqlparser,
};
use sqlex_schema::Catalog;
use sqlex_types::{ColumnDef, Dialect, ResultColumn, SqlType};
use sqlparser::ast::{JoinOperator, TableFactor};

use crate::{
    error::AnalyzeError,
    scope::{Scope, ScopeTable},
    type_inference::{DefaultTypeResolver, TypeResolver},
};

/// Result of query analysis.
#[derive(Debug, Clone)]
pub struct AnalyzeResult {
    /// Columns in the result set
    pub columns: Vec<ResultColumn>,
}

/// Query analyzer.
/// Query analyzer.
pub struct QueryAnalyzer<'a, C: Catalog> {
    catalog: &'a C,
    dialect: Dialect,
    type_resolver: Box<dyn TypeResolver>,
}

impl<'a, C: Catalog> QueryAnalyzer<'a, C> {
    /// Create a new query analyzer.
    pub fn new(catalog: &'a C, dialect: Dialect) -> Self {
        // TODO: In the future we can have different resolvers per dialect
        let type_resolver: Box<dyn TypeResolver> = Box::new(DefaultTypeResolver);
        Self {
            catalog,
            dialect,
            type_resolver,
        }
    }

    /// Analyze a SQL query string.
    pub fn analyze(&self, sql: &str) -> Result<AnalyzeResult, AnalyzeError> {
        let stmt = parse_one(sql, self.dialect)?;

        match stmt {
            Statement::Query(query) => self.analyze_query(&query),
            _ => Err(AnalyzeError::NotASelectQuery),
        }
    }

    /// Analyze a Query AST.
    pub fn analyze_query(&self, query: &Query) -> Result<AnalyzeResult, AnalyzeError> {
        self.analyze_query_context(query, &[], None)
    }

    fn analyze_query_context(
        &self,
        query: &Query,
        outer_ctes: &[&HashMap<String, ScopeTable>],
        parent_scope: Option<&Scope>,
    ) -> Result<AnalyzeResult, AnalyzeError> {
        // Local CTEs for this query level
        let mut local_ctes = HashMap::new();

        // Prepare CTE recursion stack
        let mut all_ctes = outer_ctes.to_vec();
        // We will push &local_ctes later if we have any

        if let Some(with) = &query.with {
            for cte in &with.cte_tables {
                let cte_name = cte.alias.name.value.clone();
                // recursive CTE logic...
                let result = if with.recursive {
                    match cte.query.body.as_ref() {
                        SetExpr::SetOperation { left, .. } => {
                            // For recursive CTE, we need to analyze base case first
                            // The base case can see outer CTEs but NOT itself (usually)
                            // But Postgres allows self-reference in base case? No.
                            // Base case sees outer_ctes.

                            // Recursive step sees outer_ctes + itself.

                            // Here logic was: analyze left with local_ctes.
                            // We need to construct the stack.
                            let mut current_ctes = outer_ctes.to_vec();
                            current_ctes.push(&local_ctes); // Add what we have so far

                            let base_res =
                                self.analyze_set_expr(left, &current_ctes, parent_scope)?;

                            let explicit_aliases = &cte.alias.columns;
                            let columns = base_res
                                .columns
                                .iter()
                                .enumerate()
                                .map(|(i, c)| ColumnDef {
                                    name: if i < explicit_aliases.len() {
                                        explicit_aliases[i].name.value.clone()
                                    } else {
                                        c.name.clone()
                                    },
                                    data_type: c.data_type.clone(),
                                    nullable: c.nullable,
                                    default: None,
                                    is_primary_key: false,
                                })
                                .collect();

                            let scope_table = ScopeTable {
                                alias: cte_name.clone(),
                                table_name: cte_name.clone(),
                                columns,
                                nullable_from_join: false,
                            };
                            local_ctes.insert(cte_name.clone(), scope_table);

                            let mut current_ctes = outer_ctes.to_vec();
                            current_ctes.push(&local_ctes);

                            self.analyze_query_context(&cte.query, &current_ctes, parent_scope)?
                        },
                        _ => {
                            let mut current_ctes = outer_ctes.to_vec();
                            current_ctes.push(&local_ctes);
                            self.analyze_query_context(&cte.query, &current_ctes, parent_scope)?
                        },
                    }
                } else {
                    let mut current_ctes = outer_ctes.to_vec();
                    current_ctes.push(&local_ctes);
                    self.analyze_query_context(&cte.query, &current_ctes, parent_scope)?
                };

                let explicit_aliases = &cte.alias.columns;

                let columns = result
                    .columns
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let name = if i < explicit_aliases.len() {
                            explicit_aliases[i].name.value.clone()
                        } else {
                            c.name.clone()
                        };

                        ColumnDef {
                            name,
                            data_type: c.data_type.clone(),
                            nullable: c.nullable,
                            default: None,
                            is_primary_key: false,
                        }
                    })
                    .collect();

                let scope_table = ScopeTable {
                    alias: cte_name.clone(),
                    table_name: cte_name.clone(),
                    columns,
                    nullable_from_join: false,
                };
                local_ctes.insert(cte_name, scope_table);
            }
        }

        // Update stack for body analysis
        all_ctes.push(&local_ctes);

        match query.body.as_ref() {
            SetExpr::Select(select) => self.analyze_select(select, &all_ctes, parent_scope),
            SetExpr::Query(inner) => self.analyze_query_context(inner, &all_ctes, parent_scope),
            SetExpr::SetOperation { left, .. } => {
                self.analyze_set_expr(left, &all_ctes, parent_scope)
            },
            SetExpr::Values(values) => self.analyze_values(values, parent_scope),
            _ => Err(AnalyzeError::Unsupported("SetExpr type".to_string())),
        }
    }

    fn analyze_set_expr(
        &self,
        expr: &SetExpr,
        ctes: &[&HashMap<String, ScopeTable>],
        parent_scope: Option<&Scope>,
    ) -> Result<AnalyzeResult, AnalyzeError> {
        match expr {
            SetExpr::Select(select) => self.analyze_select(select, ctes, parent_scope),
            SetExpr::Query(query) => self.analyze_query_context(query, ctes, parent_scope),
            SetExpr::SetOperation { left, .. } => self.analyze_set_expr(left, ctes, parent_scope),
            SetExpr::Values(values) => self.analyze_values(values, parent_scope),
            _ => Ok(AnalyzeResult { columns: vec![] }),
        }
    }

    fn analyze_select(
        &self,
        select: &Select,
        ctes: &[&HashMap<String, ScopeTable>],
        parent_scope: Option<&Scope>,
    ) -> Result<AnalyzeResult, AnalyzeError> {
        // Build scope from FROM clause
        let mut scope = if let Some(parent) = parent_scope {
            Scope::new_child(parent)
        } else {
            Scope::new()
        };

        self.resolve_from_clause(&select.from, &mut scope, ctes)?;

        // Analyze WHERE clause
        if let Some(selection) = &select.selection {
            self.type_resolver.infer(&scope, selection)?;
            // We should check for errors/unknown columns here, but TypeInference
            // currently returns (Unknown, true) on error rather than Result.
            // To be strict, we need TypeInference to tell us if valid.
            // For now, at least we exercise the lookups (which might trigger panics or logs if we had them).
            // But wait, TypeInference::infer calls scope.resolve_column.
            // scope.resolve_column returns Result.
            // TypeInference swallows the error.
            // We need a way to check validity.
            // Re-implementing validation or changing TypeInference is needed.
        }

        // Analyze GROUP BY
        use sqlex_parser::sqlparser::ast::GroupByExpr;
        match &select.group_by {
            GroupByExpr::All(_) => {},
            GroupByExpr::Expressions(exprs, _) => {
                for expr in exprs {
                    self.type_resolver.infer(&scope, expr)?;
                }
            },
        }

        // Analyze HAVING
        if let Some(having) = &select.having {
            self.type_resolver.infer(&scope, having)?;
        }

        // Analyze SELECT items
        let mut columns = Vec::new();

        for item in &select.projection {
            match item {
                SelectItem::UnnamedExpr(expr) => {
                    let col = self.analyze_select_expr(&scope, expr, None)?;
                    columns.push(col);
                },
                SelectItem::ExprWithAlias { expr, alias } => {
                    let col = self.analyze_select_expr(&scope, expr, Some(&alias.value))?;
                    columns.push(col);
                },
                SelectItem::QualifiedWildcard(name, _) => {
                    let table_alias = qualified_wildcard_to_string(name);
                    let table_columns = scope
                        .table_columns(&table_alias)
                        .ok_or_else(|| AnalyzeError::UnknownTable(table_alias.to_string()))?;

                    for resolved in table_columns {
                        columns.push(ResultColumn::from_table_column(
                            &resolved.column_name,
                            resolved.data_type,
                            resolved.nullable,
                            &resolved.table_name,
                            &resolved.column_name,
                        ));
                    }
                },
                SelectItem::Wildcard(_) => {
                    for resolved in scope.all_columns() {
                        columns.push(ResultColumn::from_table_column(
                            &resolved.column_name,
                            resolved.data_type,
                            resolved.nullable,
                            &resolved.table_name,
                            &resolved.column_name,
                        ));
                    }
                },
            }
        }

        Ok(AnalyzeResult { columns })
    }

    fn resolve_from_clause(
        &self,
        from: &[TableWithJoins],
        scope: &mut Scope,
        ctes: &[&HashMap<String, ScopeTable>],
    ) -> Result<(), AnalyzeError> {
        for table_with_joins in from {
            self.resolve_table_factor(&table_with_joins.relation, scope, false, ctes)?;

            for join in &table_with_joins.joins {
                let nullable = matches!(
                    join.join_operator,
                    JoinOperator::Left(_)
                        | JoinOperator::LeftOuter(_)
                        | JoinOperator::Right(_)
                        | JoinOperator::RightOuter(_)
                        | JoinOperator::FullOuter(_)
                        | JoinOperator::LeftSemi(_)
                        | JoinOperator::LeftAnti(_)
                );

                self.resolve_table_factor(&join.relation, scope, nullable, ctes)?;
            }
        }
        Ok(())
    }

    fn resolve_table_factor(
        &self,
        factor: &TableFactor,
        scope: &mut Scope,
        nullable_from_join: bool,
        ctes: &[&HashMap<String, ScopeTable>],
    ) -> Result<(), AnalyzeError> {
        match factor {
            TableFactor::Table { name, alias, .. } => {
                let table_name = object_name_to_string(name);
                let alias_name = alias.as_ref().map(|a| a.name.value.as_str());

                // CTE check (reverse order)
                for ctes_map in ctes.iter().rev() {
                    if let Some(cte_table) = ctes_map.get(&table_name) {
                        let mut scope_table = cte_table.clone();
                        if let Some(alias) = alias_name {
                            scope_table.alias = alias.to_string();
                        }
                        scope_table.nullable_from_join = nullable_from_join;
                        scope.add_table(scope_table);
                        return Ok(());
                    }
                }

                let table_def = self
                    .catalog
                    .get_table(&table_name)
                    .ok_or_else(|| AnalyzeError::UnknownTable(table_name.to_string()))?;

                let mut scope_table = ScopeTable::from_table_def(table_def, alias_name);
                scope_table.nullable_from_join = nullable_from_join;
                scope.add_table(scope_table);
            },
            TableFactor::Derived {
                lateral,
                subquery,
                alias,
            } => {
                // If LATERAL, subquery can see current SCOPE.
                // If NOT LATERAL, subquery sees PARENT scope (from outer query).
                // But `scope` here is the *current* accumulating scope.
                // We don't have direct access to `scope.parent` in public API unless we expose it.
                // But we can check `lateral` flag.

                // Oops, `Scope` transparency: if I pass `scope` (current), it has parent.
                // If I pass `scope` to `analyze_query_context` as `parent_scope`, then the subquery starts a new child of `scope`.
                // Access rules:
                // Non-LATERAL: Can ONLY access parent of current scope (outer query). Cannot access siblings.
                // LATERAL: Can access parent + siblings (current scope).

                // So:
                // If LATERAL: parent = scope.
                // If NOT LATERAL: parent = scope.parent? However `Scope` field is private.
                // But wait, `analyze_query_context` takes `parent_scope`.
                // If NOT LATERAL, we should theoretically pass the *outer* scope, not the one we are building.
                // But passing `scope` (the one being built) WOULD expose siblings, which is wrong for non-lateral.

                // Hack: `Scope` is just a struct. I can get `scope.parent` if I make it public.
                // Or I just pass `scope` if LATERAL.
                // If NOT LATERAL, I need the persistent parent.
                // `resolve_from_clause` doesn't receive `parent_scope` separately?
                // `scope` has `parent` field set in `analyze_select`.
                // So I can just access `scope.parent`.
                // I need to change `Scope::parent` to be accessible or add a getter.
                // `scope.rs` defines `parent` as private. I should make it public or add accessor.

                // Let's assume I add `pub fn parent(&self) -> Option<&Scope>` to Scope.

                // For now, I will assume non-lateral has NO parent access (simplification) OR I try to fix Scope visibility.

                // Wait, non-lateral derived tables *can* optionally be correlated to *further* outer queries?
                // Standard SQL: Derived tables in FROM are isolated. But Postgres allows lateral.
                // If it is NOT lateral, it CANNOT access tables from the same FROM clause.
                // But can it access tables from *outer* SELECT level?
                // "Subqueries in FROM cannot be correlated unless LATERAL".
                // So actually, if NOT LATERAL, it should see NOTHING from the surroundings (except global tables/functions).
                // So `parent_scope` should be `None`?
                // But it might need to resolve CTEs. `ctes` are passed separately.
                // So `Ok` to pass `None` for parent_scope if not lateral.

                let sub_parent = if *lateral { Some(&*scope) } else { None };

                let result = self.analyze_query_context(subquery, ctes, sub_parent)?;

                if let Some(alias_node) = alias {
                    let alias_name = alias_node.name.value.clone();
                    let columns = result
                        .columns
                        .iter()
                        .enumerate()
                        .map(|(i, c)| {
                            let name = if i < alias_node.columns.len() {
                                alias_node.columns[i].name.value.clone()
                            } else {
                                c.name.clone()
                            };

                            ColumnDef {
                                name,
                                data_type: c.data_type.clone(),
                                nullable: c.nullable,
                                default: None,
                                is_primary_key: false,
                            }
                        })
                        .collect();

                    let scope_table = ScopeTable {
                        alias: alias_name,
                        table_name: "derived".to_string(), // placeholder
                        columns,
                        nullable_from_join,
                    };
                    scope.add_table(scope_table);
                }
            },
            TableFactor::Function {
                name, args, alias, ..
            } => {
                // Support UNNEST(x)
                // Check function name
                let name_str = ident_to_string(name.0.last().unwrap()).to_uppercase();
                if name_str == "UNNEST" {
                    // Analyze args
                    use sqlparser::ast::{FunctionArg, FunctionArgExpr};

                    let mut elem_type = SqlType::Unknown;
                    // args is Vec<FunctionArg>
                    for arg in args {
                        if let FunctionArg::Unnamed(FunctionArgExpr::Expr(expr)) = arg {
                            let (data_type, _) = self.type_resolver.infer(scope, expr)?;
                            if let SqlType::Array(inner) = data_type {
                                elem_type = *inner;
                                break;
                            }
                        }
                    }

                    let alias_name = alias
                        .as_ref()
                        .map(|a| a.name.value.clone())
                        .unwrap_or_else(|| "unnest".to_string());

                    let col_name = if let Some(alias_node) = alias {
                        if !alias_node.columns.is_empty() {
                            alias_node.columns[0].name.value.clone()
                        } else {
                            alias_name.clone()
                        }
                    } else {
                        "unnest".to_string()
                    };

                    let scope_table = ScopeTable {
                        alias: alias_name,
                        table_name: "function".to_string(),
                        columns: vec![ColumnDef {
                            name: col_name,
                            data_type: elem_type,
                            nullable: true, // unnest can be null
                            default: None,
                            is_primary_key: false,
                        }],
                        nullable_from_join,
                    };
                    scope.add_table(scope_table);
                }
            },
            TableFactor::NestedJoin {
                table_with_joins, ..
            } => {
                self.resolve_table_factor(
                    &table_with_joins.relation,
                    scope,
                    nullable_from_join,
                    ctes,
                )?;
                for join in &table_with_joins.joins {
                    let join_nullable = nullable_from_join
                        || matches!(
                            join.join_operator,
                            JoinOperator::Left(_)
                                | JoinOperator::LeftOuter(_)
                                | JoinOperator::Right(_)
                                | JoinOperator::RightOuter(_)
                                | JoinOperator::FullOuter(_)
                        );
                    self.resolve_table_factor(&join.relation, scope, join_nullable, ctes)?;
                }
            },
            _ => {},
        }
        Ok(())
    }

    fn analyze_select_expr(
        &self,
        scope: &Scope,
        expr: &Expr,
        alias: Option<&str>,
    ) -> Result<ResultColumn, AnalyzeError> {
        // Infer type and nullability
        let (data_type, nullable) = self.type_resolver.infer(scope, expr)?;

        // Determine column name
        let (name, source_table, source_column) = match expr {
            Expr::Identifier(ident) => {
                let col_name = alias.unwrap_or(&ident.value);
                let source = scope.resolve_column(None, &ident.value).ok();
                (
                    col_name.to_string(),
                    source.as_ref().map(|s| s.table_name.clone()),
                    source.map(|s| s.column_name),
                )
            },
            Expr::CompoundIdentifier(idents) if idents.len() == 2 => {
                let table_alias = &idents[0].value;
                let col = &idents[1].value;
                let col_name = alias.unwrap_or(col);
                let source = scope.resolve_column(Some(table_alias), col).ok();
                (
                    col_name.to_string(),
                    source.as_ref().map(|s| s.table_name.clone()),
                    source.map(|s| s.column_name),
                )
            },
            _ => {
                // Expression - use alias or generate name
                let name = alias
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| self.expr_to_name(expr).into_owned());
                (name, None, None)
            },
        };

        let mut result = ResultColumn::new(name, data_type, nullable);
        if let Some(table) = source_table {
            result = result.with_source_table(table);
        }
        if let Some(column) = source_column {
            result = result.with_source_column(column);
        }

        Ok(result)
    }

    fn analyze_values(
        &self,
        values: &sqlparser::ast::Values,
        parent_scope: Option<&Scope>,
    ) -> Result<AnalyzeResult, AnalyzeError> {
        if values.rows.is_empty() {
            return Ok(AnalyzeResult { columns: vec![] });
        }

        let first_row = &values.rows[0];
        let mut columns = Vec::new();
        // Create scope for expression analysis
        let scope = if let Some(p) = parent_scope {
            Scope::new_child(p)
        } else {
            Scope::new()
        };

        for (i, expr) in first_row.iter().enumerate() {
            let (data_type, nullable) = self.type_resolver.infer(&scope, expr)?;
            let name = if self.dialect == Dialect::MySQL {
                format!("column_{}", i)
            } else {
                format!("column{}", i + 1)
            };
            columns.push(ResultColumn::new(name, data_type, nullable));
        }

        // Validate subsequent rows
        for (row_idx, row) in values.rows.iter().enumerate().skip(1) {
            if row.len() != first_row.len() {
                return Err(AnalyzeError::InvalidQuery(
                    "VALUES rows have different number of columns".to_string(),
                ));
            }
            for (col_idx, expr) in row.iter().enumerate() {
                let (t, _) = self.type_resolver.infer(&scope, expr)?;
                if !t.is_compatible(&columns[col_idx].data_type) {
                    return Err(AnalyzeError::TypeMismatch(format!(
                        "Row {} Column {} has type {:?}, but expected {:?}",
                        row_idx, col_idx, t, columns[col_idx].data_type
                    )));
                }
            }
        }

        Ok(AnalyzeResult { columns })
    }

    /// Generate a name for an expression (fallback when no alias).
    fn expr_to_name(&self, expr: &Expr) -> std::borrow::Cow<'static, str> {
        match expr {
            Expr::Identifier(ident) => std::borrow::Cow::Owned(ident.value.clone()),
            Expr::CompoundIdentifier(idents) => idents
                .last()
                .map(|i| std::borrow::Cow::Owned(i.value.clone()))
                .unwrap_or(std::borrow::Cow::Borrowed("?column?")),
            _ if self.dialect == Dialect::MySQL => {
                // In MySQL, default names are often the expression itself
                std::borrow::Cow::Owned(expr.to_string())
            },
            Expr::Function(func) => func
                .name
                .0
                .last()
                .map(|i| std::borrow::Cow::Owned(ident_to_string(i)))
                .unwrap_or(std::borrow::Cow::Borrowed("?column?")),
            Expr::Value(_) => std::borrow::Cow::Borrowed("?column?"),
            Expr::BinaryOp { .. } => std::borrow::Cow::Borrowed("?column?"),
            Expr::UnaryOp { .. } => std::borrow::Cow::Borrowed("?column?"),
            Expr::Cast { .. } => std::borrow::Cow::Borrowed("?column?"),
            Expr::Case { .. } => std::borrow::Cow::Borrowed("case"),
            _ => std::borrow::Cow::Borrowed("?column?"),
        }
    }
}

fn object_name_to_string(name: &sqlparser::ast::ObjectName) -> String {
    name.0.last().map(ident_to_string).unwrap_or_default()
}

/// Extract table name from QualifiedWildcard
fn qualified_wildcard_to_string(kind: &sqlparser::ast::SelectItemQualifiedWildcardKind) -> String {
    match kind {
        sqlparser::ast::SelectItemQualifiedWildcardKind::ObjectName(name) => {
            name.0.last().map(ident_to_string).unwrap_or_default()
        },
        sqlparser::ast::SelectItemQualifiedWildcardKind::Expr(_) => String::new(),
    }
}

fn ident_to_string(ident: &sqlparser::ast::ObjectNamePart) -> String {
    match ident {
        sqlparser::ast::ObjectNamePart::Identifier(id) => id.value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use sqlex_schema::SchemaRegistry;
    use sqlex_types::SqlType;

    use super::*;

    fn create_test_schema() -> SchemaRegistry {
        let mut registry = SchemaRegistry::new(Dialect::PostgreSQL);
        registry
            .apply_sql(
                r#"
                CREATE TABLE users (
                    id SERIAL PRIMARY KEY,
                    name VARCHAR(100) NOT NULL,
                    email VARCHAR(100),
                    created_at TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE TABLE orders (
                    id SERIAL PRIMARY KEY,
                    user_id INTEGER NOT NULL,
                    amount DECIMAL(10, 2) NOT NULL,
                    status VARCHAR(20)
                );
                "#,
            )
            .unwrap();
        registry
    }

    #[test]
    fn test_simple_select() {
        let registry = create_test_schema();
        let analyzer = QueryAnalyzer::new(&registry, Dialect::PostgreSQL);

        let result = analyzer.analyze("SELECT id, name FROM users").unwrap();
        assert_eq!(result.columns.len(), 2);
        assert_eq!(result.columns[0].name, "id");
        assert_eq!(result.columns[0].data_type, SqlType::Integer);
        assert!(!result.columns[0].nullable);
        assert_eq!(result.columns[1].name, "name");
        assert!(!result.columns[1].nullable);
    }

    #[test]
    fn test_select_with_alias() {
        let registry = create_test_schema();
        let analyzer = QueryAnalyzer::new(&registry, Dialect::PostgreSQL);

        let result = analyzer
            .analyze("SELECT id AS user_id, name AS user_name FROM users")
            .unwrap();
        assert_eq!(result.columns[0].name, "user_id");
        assert_eq!(result.columns[1].name, "user_name");
    }

    #[test]
    fn test_select_star() {
        let registry = create_test_schema();
        let analyzer = QueryAnalyzer::new(&registry, Dialect::PostgreSQL);

        let result = analyzer.analyze("SELECT * FROM users").unwrap();
        assert_eq!(result.columns.len(), 4);
    }

    #[test]
    fn test_select_with_expression() {
        let registry = create_test_schema();
        let analyzer = QueryAnalyzer::new(&registry, Dialect::PostgreSQL);

        let result = analyzer
            .analyze("SELECT id, amount * 2 AS double_amount FROM orders")
            .unwrap();
        assert_eq!(result.columns.len(), 2);
        assert_eq!(result.columns[1].name, "double_amount");
    }

    #[test]
    fn test_select_with_aggregate() {
        let registry = create_test_schema();
        let analyzer = QueryAnalyzer::new(&registry, Dialect::PostgreSQL);

        let result = analyzer
            .analyze("SELECT COUNT(*) AS cnt, SUM(amount) AS total FROM orders")
            .unwrap();
        assert_eq!(result.columns.len(), 2);
        assert_eq!(result.columns[0].name, "cnt");
        assert_eq!(result.columns[0].data_type, SqlType::BigInt);
        assert!(!result.columns[0].nullable); // COUNT is never null
        assert_eq!(result.columns[1].name, "total");
        assert!(result.columns[1].nullable); // SUM can be null for empty set
    }

    #[test]
    fn test_left_join_nullability() {
        let registry = create_test_schema();
        let analyzer = QueryAnalyzer::new(&registry, Dialect::PostgreSQL);

        let result = analyzer
            .analyze(
                "SELECT u.id, u.name, o.amount 
                 FROM users u 
                 LEFT JOIN orders o ON u.id = o.user_id",
            )
            .unwrap();

        // users columns should retain their nullability
        assert!(!result.columns[0].nullable); // u.id (PK, not null)
        assert!(!result.columns[1].nullable); // u.name (NOT NULL)
        // orders columns should be nullable due to LEFT JOIN
        assert!(result.columns[2].nullable); // o.amount (nullable from join)
    }

    #[test]
    fn test_cte_scoping() {
        let registry = create_test_schema();
        let analyzer = QueryAnalyzer::new(&registry, Dialect::PostgreSQL);

        let sql = "
            WITH user_stats AS (
                SELECT user_id, COUNT(*) as order_count 
                FROM orders 
                GROUP BY user_id
            )
            SELECT u.name, s.order_count
            FROM users u
            JOIN user_stats s ON u.id = s.user_id
        ";

        let result = analyzer.analyze(sql).unwrap();
        assert_eq!(result.columns.len(), 2);
        assert_eq!(result.columns[0].name, "name");
        assert_eq!(result.columns[1].name, "order_count");
        assert_eq!(result.columns[1].data_type, SqlType::BigInt);
    }
}
