use std::collections::HashMap;

use sqlex_common::dialect::Dialect;
use sqlparser::{
    ast::{
        Expr, GroupByExpr, Query, Select, SelectItem, SetExpr, SetOperator, SetQuantifier,
        Statement, Value,
    },
    dialect::{Dialect as SqlParserDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect},
    parser::Parser,
};

use crate::{
    analysis::{bind::scope::BindScope, diagnostics::Diagnostic, keywords},
    catalog::Catalog,
    ir::{
        arena::Arena,
        bound::{
            BoundColumn, BoundExpr, BoundOrderBy, BoundProjection, BoundQueryBody, BoundSelect,
            BoundSetExpr, BoundSetOp, BoundStatement, BoundTable,
        },
        ids::{ColumnId, ExprId, TableId},
    },
};

mod cte;
mod expr;
mod from;
mod names;
pub(crate) mod scope;

pub struct BindResult {
    pub bound: Option<BoundStatement>,
    pub diagnostics: Vec<Diagnostic>,
}

pub(crate) struct Binder<'a> {
    pub(super) dialect: Dialect,
    pub(super) catalog: &'a Catalog,
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) cte_scope: HashMap<String, CteBinding>,
    pub(super) cte_defs: Vec<CteDefEntry>,
    pub(super) outer_scopes: Vec<BindScope>,
    pub(super) tables: Arena<BoundTable, TableId>,
    pub(super) columns: Arena<BoundColumn, ColumnId>,
    pub(super) exprs: Arena<BoundExpr, ExprId>,
}

#[derive(Debug, Clone)]
pub(super) struct CteBinding {
    pub(super) columns: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct CteDefEntry {
    pub(super) name: String,
    pub(super) columns: Vec<String>,
    pub(super) query: BoundQueryBody,
    pub(super) recursive: bool,
}

impl<'a> Binder<'a> {
    pub fn new(dialect: Dialect, catalog: &'a Catalog) -> Self {
        Self {
            dialect,
            catalog,
            diagnostics: Vec::new(),
            cte_scope: HashMap::new(),
            cte_defs: Vec::new(),
            outer_scopes: Vec::new(),
            tables: Arena::default(),
            columns: Arena::default(),
            exprs: Arena::default(),
        }
    }

    pub fn bind(mut self, sql: &str) -> BindResult {
        let result = self.bind_sql(sql);
        match result {
            Some((ctes, query_body)) => {
                let stmt = BoundStatement {
                    tables: self.tables,
                    columns: self.columns,
                    exprs: self.exprs,
                    ctes,
                    query: query_body,
                };
                BindResult {
                    bound: Some(stmt),
                    diagnostics: self.diagnostics,
                }
            },
            None => BindResult {
                bound: None,
                diagnostics: self.diagnostics,
            },
        }
    }

    fn bind_sql(&mut self, sql: &str) -> Option<(Vec<crate::ir::bound::BoundCte>, BoundQueryBody)> {
        let dialect: Box<dyn SqlParserDialect> = match self.dialect {
            Dialect::Postgres => Box::new(PostgreSqlDialect {}),
            Dialect::MySQL => Box::new(MySqlDialect {}),
            Dialect::SQLite => Box::new(SQLiteDialect {}),
        };
        let statements = match Parser::parse_sql(dialect.as_ref(), sql) {
            Ok(stmts) => stmts,
            Err(e) => {
                self.diagnostics
                    .push(Diagnostic::parse_error(format!("Parse error: {e}")));
                return None;
            },
        };

        if statements.len() != 1 {
            self.diagnostics.push(Diagnostic::invalid_statement(
                "Expected exactly one statement",
            ));
            return None;
        }

        match &statements[0] {
            Statement::Query(query) => self.bind_query(query),
            _ => {
                self.diagnostics
                    .push(Diagnostic::invalid_statement("Expected a SELECT query"));
                None
            },
        }
    }

    pub(super) fn bind_query(
        &mut self,
        query: &Query,
    ) -> Option<(Vec<crate::ir::bound::BoundCte>, BoundQueryBody)> {
        let cte_entries = self.bind_ctes(query.with.as_ref());

        let (body, order_scope, alias_map, projection_exprs) = match &*query.body {
            SetExpr::Select(select) => {
                let (select, scope, aliases) = self.bind_select(select);
                let projection_exprs = select.projection.iter().map(|p| p.expr).collect::<Vec<_>>();
                (
                    BoundSetExpr::Select(select),
                    Some(scope),
                    aliases,
                    projection_exprs,
                )
            },
            other => (self.bind_set_expr(other), None, HashMap::new(), Vec::new()),
        };

        let mut order_by = Vec::new();
        if let Some(order_by_clause) = &query.order_by {
            for ob in &order_by_clause.exprs {
                let expr_id = self.bind_order_by_expr(
                    &ob.expr,
                    order_scope.as_ref(),
                    &alias_map,
                    &projection_exprs,
                );
                order_by.push(BoundOrderBy {
                    expr: expr_id,
                    asc: ob.asc.unwrap_or(true),
                    nulls_first: ob.nulls_first,
                });
            }
        }

        let limit = query
            .limit
            .as_ref()
            .map(|e| self.bind_expr(e, &BindScope::default()));
        let offset = query
            .offset
            .as_ref()
            .map(|o| self.bind_expr(&o.value, &BindScope::default()));

        let ctes = cte_entries
            .into_iter()
            .map(|e| crate::ir::bound::BoundCte {
                name: e.name,
                columns: e.columns,
                query: e.query,
                recursive: e.recursive,
            })
            .collect();

        let query_body = BoundQueryBody {
            body,
            order_by,
            limit,
            offset,
        };

        Some((ctes, query_body))
    }

    pub(super) fn bind_query_body(&mut self, query: &Query) -> BoundQueryBody {
        match self.bind_query(query) {
            Some((_ctes, body)) => body,
            None => BoundQueryBody {
                body: BoundSetExpr::Values { rows: Vec::new() },
                order_by: Vec::new(),
                limit: None,
                offset: None,
            },
        }
    }

    pub(super) fn bind_set_expr(&mut self, set_expr: &SetExpr) -> BoundSetExpr {
        match set_expr {
            SetExpr::Select(select) => BoundSetExpr::Select(self.bind_select(select).0),
            SetExpr::Query(query) => BoundSetExpr::Query(Box::new(self.bind_query_body(query))),
            SetExpr::SetOperation {
                op,
                left,
                right,
                set_quantifier,
            } => {
                let left_expr = self.bind_set_expr(left);
                let right_expr = self.bind_set_expr(right);
                BoundSetExpr::SetOperation {
                    op: map_set_op(op),
                    all: matches!(
                        set_quantifier,
                        SetQuantifier::All | SetQuantifier::AllByName
                    ),
                    left: Box::new(left_expr),
                    right: Box::new(right_expr),
                }
            },
            SetExpr::Values(values) => {
                let mut rows = Vec::new();
                for row in &values.rows {
                    let mut bound_row = Vec::new();
                    for expr in row {
                        bound_row.push(self.bind_expr(expr, &BindScope::default()));
                    }
                    rows.push(bound_row);
                }
                BoundSetExpr::Values { rows }
            },
            _ => {
                self.diagnostics
                    .push(Diagnostic::unsupported_feature("set expression in binder"));
                BoundSetExpr::Values { rows: Vec::new() }
            },
        }
    }

    fn bind_select(
        &mut self,
        select: &Select,
    ) -> (BoundSelect, BindScope, HashMap<String, ExprId>) {
        let (from, scope) = self.bind_from(&select.from);

        let selection = select
            .selection
            .as_ref()
            .map(|expr| self.bind_expr(expr, &scope));

        let mut projection = Vec::new();
        let mut alias_map = HashMap::new();
        for item in &select.projection {
            match item {
                SelectItem::UnnamedExpr(expr) => {
                    let expr_id = self.bind_expr(expr, &scope);
                    projection.push(BoundProjection {
                        expr: expr_id,
                        alias: None,
                    });
                },
                SelectItem::ExprWithAlias { expr, alias } => {
                    let expr_id = self.bind_expr(expr, &scope);
                    if keywords::is_reserved_identifier(self.dialect, alias) {
                        self.diagnostics.push(Diagnostic::invalid_statement(format!(
                            "Alias {alias} is a reserved keyword in {dialect}; quote it to use as an identifier",
                            dialect = self.dialect
                        )));
                    }
                    alias_map.insert(alias.value.clone(), expr_id);
                    projection.push(BoundProjection {
                        expr: expr_id,
                        alias: Some(alias.value.clone()),
                    });
                },
                SelectItem::Wildcard(_) => {
                    for col in scope.columns_in_order() {
                        projection.push(BoundProjection {
                            expr: self.exprs.alloc(BoundExpr::Column(col.id)),
                            alias: Some(col.name.clone()),
                        });
                    }
                },
                SelectItem::QualifiedWildcard(name, _) => {
                    let alias = name.to_string();
                    match scope.columns_for_table(&alias) {
                        Some(cols) => {
                            for col in cols {
                                projection.push(BoundProjection {
                                    expr: self.exprs.alloc(BoundExpr::Column(col.id)),
                                    alias: Some(col.name.clone()),
                                });
                            }
                        },
                        None => {
                            self.diagnostics.push(
                                Diagnostic::unknown_table_alias(&alias)
                                    .with_context("SELECT qualified wildcard"),
                            );
                        },
                    }
                },
            }
        }

        let mut group_by = Vec::new();
        match &select.group_by {
            GroupByExpr::Expressions(exprs, _) => {
                for expr in exprs {
                    group_by.push(self.bind_expr(expr, &scope));
                }
            },
            GroupByExpr::All(_) => {
                self.diagnostics
                    .push(Diagnostic::unsupported_feature("GROUP BY ALL"));
            },
        }

        let having = select
            .having
            .as_ref()
            .map(|expr| self.bind_expr(expr, &scope));

        let bound = BoundSelect {
            from,
            projection,
            selection,
            group_by,
            having,
            distinct: select.distinct.is_some(),
        };

        (bound, scope, alias_map)
    }

    fn bind_order_by_expr(
        &mut self,
        expr: &Expr,
        scope: Option<&BindScope>,
        alias_map: &HashMap<String, ExprId>,
        projection_exprs: &[ExprId],
    ) -> ExprId {
        if let Expr::Identifier(ident) = expr {
            if let Some(expr_id) = alias_map.get(&ident.value) {
                return *expr_id;
            }
        }

        if let Expr::Value(Value::Number(raw, _)) = expr {
            if let Ok(index) = raw.parse::<usize>() {
                if index >= 1 && index <= projection_exprs.len() {
                    return projection_exprs[index - 1];
                }
                self.diagnostics
                    .push(Diagnostic::order_by_position_out_of_range(index));
            }
        }

        match scope {
            Some(scope) => self.bind_expr(expr, scope),
            None => self.bind_expr(expr, &BindScope::default()),
        }
    }
}

fn map_set_op(op: &SetOperator) -> BoundSetOp {
    match op {
        SetOperator::Union => BoundSetOp::Union,
        SetOperator::Intersect => BoundSetOp::Intersect,
        SetOperator::Except => BoundSetOp::Except,
        _ => BoundSetOp::Union,
    }
}
