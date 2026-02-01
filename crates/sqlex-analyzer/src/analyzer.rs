//! Main query analyzer.

use std::collections::HashMap;

use sqlex_parser::{Expr, Query, Select, SelectItem, SetExpr, Statement, parse_one, sqlparser};
use sqlex_schema::SchemaRegistry;
use sqlex_types::{ColumnDef, Dialect, ResultColumn};

use crate::{
    error::AnalyzeError,
    resolver::FromResolver,
    scope::{Scope, ScopeTable},
    type_inference::TypeInference,
};

/// Result of query analysis.
#[derive(Debug, Clone)]
pub struct AnalyzeResult {
    /// Columns in the result set
    pub columns: Vec<ResultColumn>,
}

/// Query analyzer.
pub struct QueryAnalyzer<'a> {
    registry: &'a SchemaRegistry,
    dialect: Dialect,
}

impl<'a> QueryAnalyzer<'a> {
    /// Create a new query analyzer.
    pub fn new(registry: &'a SchemaRegistry, dialect: Dialect) -> Self {
        Self { registry, dialect }
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
        self.analyze_query_context(query, None)
    }

    fn analyze_query_context(
        &self,
        query: &Query,
        outer_ctes: Option<&HashMap<String, ScopeTable>>,
    ) -> Result<AnalyzeResult, AnalyzeError> {
        // Collect CTEs
        let mut local_ctes = HashMap::new();
        if let Some(ctes) = outer_ctes {
            local_ctes.extend(ctes.clone());
        }

        if let Some(with) = &query.with {
            for cte in &with.cte_tables {
                let cte_name = cte.alias.name.value.clone();
                // Analyze CTE query
                // For recursive CTEs, we might need to seed the table first.
                // Simple recursive support: assume schema from non-recursive part or just ignore self-ref for now if it fails.
                // However, without proper recursive handling, the self-ref lookup will fail.
                // A complete fix requires parsing the CTE query to find the UNION/non-recursive part, analyzing it, adding to scope, then analyzing recursive part.
                // For now, let's try analyzing the query with current scope.

                // Note: If recursive, we should bind the table name to the scope for the query itself.
                // But we don't know the schema yet.
                // If it's recursive, the query body is usually a SetOperation (UNION).

                let result = if with.recursive {
                    // Hack for recursive CTEs:
                    // 1. Analyze ignoring the recursive part/self-ref might be hard blindly.
                    // 2. Or, we can proceed and catch "UnknownTable" and then retry? No.
                    // Correct way: Extract schema from the left side of UNION.
                    match cte.query.body.as_ref() {
                        SetExpr::SetOperation { left, .. } => {
                            // Analyze left part (non-recursive base case)
                            // For the base case, we pass the outer CTEs (and any defined before this one)
                            let base_res = self.analyze_set_expr(left, Some(&local_ctes))?;

                            // Add to scope so right side can see it
                            let columns = base_res
                                .columns
                                .iter()
                                .map(|c| ColumnDef {
                                    name: c.name.clone(),
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

                            // Now analyze the full query (including right side) with the self-ref in scope
                            self.analyze_query_context(&cte.query, Some(&local_ctes))?
                        },
                        _ => {
                            // Not a standard recursive CTE structure? Just analyze normally.
                            self.analyze_query_context(&cte.query, Some(&local_ctes))?
                        },
                    }
                } else {
                    self.analyze_query_context(&cte.query, Some(&local_ctes))?
                };

                let columns = result
                    .columns
                    .iter()
                    .map(|c| ColumnDef {
                        name: c.name.clone(),
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
                local_ctes.insert(cte_name, scope_table);
            }
        }

        match query.body.as_ref() {
            SetExpr::Select(select) => self.analyze_select(select, Some(&local_ctes)),
            SetExpr::Query(inner) => self.analyze_query_context(inner, Some(&local_ctes)),
            SetExpr::SetOperation { left, .. } => {
                // For UNION/INTERSECT/EXCEPT, use the left side's columns
                self.analyze_set_expr(left, Some(&local_ctes))
            },
            SetExpr::Values(_) => {
                // VALUES clause - would need value type inference
                Ok(AnalyzeResult { columns: vec![] })
            },
            _ => Err(AnalyzeError::Unsupported("SetExpr type".to_string())),
        }
    }

    fn analyze_set_expr(
        &self,
        expr: &SetExpr,
        ctes: Option<&HashMap<String, ScopeTable>>,
    ) -> Result<AnalyzeResult, AnalyzeError> {
        match expr {
            SetExpr::Select(select) => self.analyze_select(select, ctes),
            SetExpr::Query(query) => self.analyze_query_context(query, ctes),
            SetExpr::SetOperation { left, .. } => self.analyze_set_expr(left, ctes),
            _ => Ok(AnalyzeResult { columns: vec![] }),
        }
    }

    fn analyze_select(
        &self,
        select: &Select,
        ctes: Option<&HashMap<String, ScopeTable>>,
    ) -> Result<AnalyzeResult, AnalyzeError> {
        // Build scope from FROM clause
        let resolver = FromResolver::new(self.registry, ctes);
        let scope = resolver.resolve(&select.from)?;

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
                    // table.* - extract table name from the qualified wildcard
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
                    // SELECT *
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

    fn analyze_select_expr(
        &self,
        scope: &Scope,
        expr: &Expr,
        alias: Option<&str>,
    ) -> Result<ResultColumn, AnalyzeError> {
        // Infer type and nullability
        let (data_type, nullable) = TypeInference::infer(scope, expr);

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
                    .unwrap_or_else(|| expr_to_name(expr));
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
}

/// Extract table name from QualifiedWildcard
fn qualified_wildcard_to_string(kind: &sqlparser::ast::SelectItemQualifiedWildcardKind) -> String {
    match kind {
        sqlparser::ast::SelectItemQualifiedWildcardKind::ObjectName(name) => name
            .0
            .last()
            .map(|i| ident_to_string(i))
            .unwrap_or_default(),
        sqlparser::ast::SelectItemQualifiedWildcardKind::Expr(_) => String::new(),
    }
}

fn ident_to_string(ident: &sqlparser::ast::ObjectNamePart) -> String {
    match ident {
        sqlparser::ast::ObjectNamePart::Identifier(id) => id.value.clone(),
    }
}

/// Generate a name for an expression (fallback when no alias).
fn expr_to_name(expr: &Expr) -> String {
    match expr {
        Expr::Identifier(ident) => ident.value.clone(),
        Expr::CompoundIdentifier(idents) => idents
            .last()
            .map(|i| i.value.clone())
            .unwrap_or_else(|| "?column?".to_string()),
        Expr::Function(func) => func
            .name
            .0
            .last()
            .map(|i| ident_to_string(i))
            .unwrap_or_else(|| "?column?".to_string()),
        Expr::Value(_) => "?column?".to_string(),
        Expr::BinaryOp { .. } => "?column?".to_string(),
        Expr::UnaryOp { .. } => "?column?".to_string(),
        Expr::Cast { .. } => "?column?".to_string(),
        Expr::Case { .. } => "case".to_string(),
        _ => "?column?".to_string(),
    }
}

#[cfg(test)]
mod tests {
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
}
