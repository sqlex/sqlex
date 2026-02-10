use sqlex_analyzer::extension::ObjectNameExt;
use sqlparser::ast::{JoinConstraint, JoinOperator, TableFactor, TableWithJoins};

use super::Algebraizer;
use crate::{
    diagnostics::{Diagnostic, DiagnosticCode},
    ir::{
        auxiliary::{JoinCondition, JoinKind},
        relational::RelationalExpr,
    },
    keywords,
};

impl<'a> Algebraizer<'a> {
    pub(super) fn build_from(&mut self, from: &[TableWithJoins]) -> Option<RelationalExpr> {
        if from.is_empty() {
            return None;
        }

        let mut result: Option<RelationalExpr> = None;

        for item in from {
            let expr = self.build_table_with_joins(item)?;
            result = Some(match result {
                None => expr,
                Some(left) => RelationalExpr::Join {
                    left: Box::new(left),
                    right: Box::new(expr),
                    kind: JoinKind::Cross,
                    condition: None,
                },
            });
        }

        result
    }

    fn build_table_with_joins(&mut self, item: &TableWithJoins) -> Option<RelationalExpr> {
        let mut expr = self.build_table_factor(&item.relation)?;

        for join in &item.joins {
            let right = self.build_table_factor(&join.relation)?;
            let (kind, condition) = self.build_join_operator(&join.join_operator);
            expr = RelationalExpr::Join {
                left: Box::new(expr),
                right: Box::new(right),
                kind,
                condition,
            };
        }

        Some(expr)
    }

    fn build_join_operator(
        &mut self,
        join_operator: &JoinOperator,
    ) -> (JoinKind, Option<JoinCondition>) {
        let (kind, constraint) = match join_operator {
            JoinOperator::Inner(constraint) => (JoinKind::Inner, Some(constraint)),
            JoinOperator::LeftOuter(constraint) => (JoinKind::Left, Some(constraint)),
            JoinOperator::RightOuter(constraint) => (JoinKind::Right, Some(constraint)),
            JoinOperator::FullOuter(constraint) => {
                if self.dialect == sqlex_common::dialect::Dialect::MySQL {
                    self.diagnostics.push(Diagnostic::error_with_code(
                        DiagnosticCode::InvalidJoin,
                        "FULL JOIN is not supported in MySQL",
                    ));
                }
                (JoinKind::Full, Some(constraint))
            },
            JoinOperator::CrossJoin => (JoinKind::Cross, None),
            _ => {
                self.diagnostics
                    .push(Diagnostic::unsupported_feature("join type in algebraizer"));
                (JoinKind::Inner, None)
            },
        };

        let condition = constraint.and_then(|c| self.build_join_condition(c));
        (kind, condition)
    }

    fn build_join_condition(&mut self, constraint: &JoinConstraint) -> Option<JoinCondition> {
        match constraint {
            JoinConstraint::On(expr) => {
                let scalar = self.build_scalar_expr(expr);
                Some(JoinCondition::On(scalar))
            },
            JoinConstraint::Using(names) => {
                let columns = names
                    .iter()
                    .map(|name| {
                        name.0
                            .last()
                            .map(|ident| ident.value.clone())
                            .unwrap_or_else(|| name.to_string())
                    })
                    .collect();
                Some(JoinCondition::Using(columns))
            },
            JoinConstraint::Natural => Some(JoinCondition::Natural),
            JoinConstraint::None => {
                self.diagnostics
                    .push(Diagnostic::unsupported_feature("JOIN constraint NONE"));
                None
            },
        }
    }

    fn build_table_factor(&mut self, table: &TableFactor) -> Option<RelationalExpr> {
        match table {
            TableFactor::Table { name, alias, .. } => {
                let table_name = name.to_normalized_string(self.dialect);
                let alias_name = alias.as_ref().map(|a| a.name.value.clone());

                if let Some(a) = alias.as_ref() {
                    if keywords::is_reserved_identifier(self.dialect, &a.name) {
                        self.diagnostics.push(Diagnostic::invalid_statement(format!(
                            "Table alias {} is a reserved keyword in {}; quote it to use as an identifier",
                            a, self.dialect
                        )));
                    }
                }

                // Check CTE scope first
                if let Some(cte_entry) = self.cte_scope.get(&table_name) {
                    let expr = cte_entry.expr.clone();
                    let label = alias_name.unwrap_or_else(|| table_name.clone());
                    return Some(RelationalExpr::Alias {
                        input: Box::new(expr),
                        name: label,
                    });
                }

                // Check catalog
                if self.catalog.get_table(&table_name).is_none() {
                    self.diagnostics
                        .push(Diagnostic::unknown_table(&table_name));
                }

                Some(RelationalExpr::Scan {
                    table: table_name,
                    alias: alias_name,
                })
            },
            TableFactor::Derived {
                subquery, alias, ..
            } => self.build_derived_table(subquery, alias.as_ref()),
            _ => {
                self.diagnostics.push(Diagnostic::unsupported_feature(
                    "table factor in algebraizer",
                ));
                Some(RelationalExpr::Scan {
                    table: "<unknown>".to_string(),
                    alias: None,
                })
            },
        }
    }

    fn build_derived_table(
        &mut self,
        subquery: &sqlparser::ast::Query,
        alias: Option<&sqlparser::ast::TableAlias>,
    ) -> Option<RelationalExpr> {
        let alias_name = alias.map(|a| a.name.value.clone());

        if alias_name.is_none() {
            self.diagnostics
                .push(Diagnostic::derived_table_requires_alias());
        }

        if let Some(a) = alias {
            if keywords::is_reserved_identifier(self.dialect, &a.name) {
                self.diagnostics.push(Diagnostic::invalid_statement(format!(
                    "Table alias {} is a reserved keyword in {}; quote it to use as an identifier",
                    a, self.dialect
                )));
            }
            for column_alias in &a.columns {
                if keywords::is_reserved_identifier(self.dialect, &column_alias.name) {
                    self.diagnostics.push(Diagnostic::invalid_statement(format!(
                        "Derived column alias {} is a reserved keyword in {}; quote it to use as an identifier",
                        column_alias.name, self.dialect
                    )));
                }
            }
        }

        let expr = self.algebraize_query(subquery)?;

        // Wrap in Alias so the derived table's columns are labeled with the alias
        if let Some(name) = alias_name {
            Some(RelationalExpr::Alias {
                input: Box::new(expr),
                name,
            })
        } else {
            Some(expr)
        }
    }
}
