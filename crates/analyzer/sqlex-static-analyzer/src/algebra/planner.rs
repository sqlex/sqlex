use sqlex_common::dialect::Dialect;
use sqlparser::ast::{Expr, Statement, UnaryOperator, Value};

mod bind_expr;
mod context;
mod cte;
mod from_join;
mod from_table_factor;
mod join;
mod select;
mod set_ops;

use crate::{
    algebra::{
        expr::{LimitNode, RelExpr},
        planner::context::BuildContext,
        scalar::OutputSchema,
    },
    catalog::model::Catalog,
    diagnostics::{Diagnostic, Phase},
    functions::registry::FunctionRegistry,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Algebraizer {
    dialect: Dialect,
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
