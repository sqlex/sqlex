//! FROM clause resolver.

use sqlex_parser::{TableFactor, TableWithJoins, sqlparser};
use sqlex_schema::SchemaRegistry;

use crate::{
    error::AnalyzeError,
    scope::{Scope, ScopeTable},
};

/// Resolve FROM clause to build a scope.
pub struct FromResolver<'a> {
    registry: &'a SchemaRegistry,
    ctes: Option<&'a std::collections::HashMap<String, ScopeTable>>,
}

impl<'a> FromResolver<'a> {
    pub fn new(
        registry: &'a SchemaRegistry,
        ctes: Option<&'a std::collections::HashMap<String, ScopeTable>>,
    ) -> Self {
        Self { registry, ctes }
    }

    /// Resolve FROM clause tables and joins.
    pub fn resolve(&self, from: &[TableWithJoins]) -> Result<Scope, AnalyzeError> {
        let mut scope = Scope::new();

        for table_with_joins in from {
            // Resolve the base table
            self.resolve_table_factor(&table_with_joins.relation, &mut scope, false)?;

            // Resolve joins
            for join in &table_with_joins.joins {
                // LEFT JOIN makes the right side nullable
                // RIGHT JOIN makes the left side nullable (but we're adding it here)
                // FULL OUTER JOIN makes both sides nullable
                let nullable = matches!(
                    join.join_operator,
                    sqlparser::ast::JoinOperator::Left(_)
                        | sqlparser::ast::JoinOperator::LeftOuter(_)
                        | sqlparser::ast::JoinOperator::Right(_)
                        | sqlparser::ast::JoinOperator::RightOuter(_)
                        | sqlparser::ast::JoinOperator::FullOuter(_)
                        | sqlparser::ast::JoinOperator::LeftSemi(_)
                        | sqlparser::ast::JoinOperator::LeftAnti(_)
                );

                self.resolve_table_factor(&join.relation, &mut scope, nullable)?;
            }
        }

        Ok(scope)
    }

    fn resolve_table_factor(
        &self,
        factor: &TableFactor,
        scope: &mut Scope,
        nullable_from_join: bool,
    ) -> Result<(), AnalyzeError> {
        match factor {
            TableFactor::Table { name, alias, .. } => {
                let table_name = object_name_to_string(name);
                let alias_name = alias.as_ref().map(|a| a.name.value.as_str());

                // Try to find in CTEs first
                if let Some(ctes) = self.ctes {
                    if let Some(cte_table) = ctes.get(&table_name) {
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
                    .registry
                    .get_table(&table_name)
                    .ok_or_else(|| AnalyzeError::UnknownTable(table_name.to_string()))?;

                let mut scope_table = ScopeTable::from_table_def(table_def, alias_name);
                scope_table.nullable_from_join = nullable_from_join;
                scope.add_table(scope_table);
            },
            TableFactor::Derived { alias, .. } => {
                // TODO: Handle subqueries by analyzing them recursively
                // For now, skip with a placeholder
                if let Some(_alias) = alias {
                    // We would need to analyze the subquery and create a scope table
                    // from its result columns
                }
            },
            TableFactor::NestedJoin {
                table_with_joins, ..
            } => {
                // Recursively resolve nested joins
                self.resolve_table_factor(&table_with_joins.relation, scope, nullable_from_join)?;
                for join in &table_with_joins.joins {
                    let join_nullable = nullable_from_join
                        || matches!(
                            join.join_operator,
                            sqlparser::ast::JoinOperator::Left(_)
                                | sqlparser::ast::JoinOperator::LeftOuter(_)
                                | sqlparser::ast::JoinOperator::Right(_)
                                | sqlparser::ast::JoinOperator::RightOuter(_)
                                | sqlparser::ast::JoinOperator::FullOuter(_)
                        );
                    self.resolve_table_factor(&join.relation, scope, join_nullable)?;
                }
            },
            _ => {
                // Other table factors (UNNEST, etc.) - skip for now
            },
        }
        Ok(())
    }
}

fn object_name_to_string(name: &sqlex_parser::ObjectName) -> String {
    name.0
        .last()
        .map(|i| ident_to_string(i))
        .unwrap_or_default()
}

fn ident_to_string(ident: &sqlparser::ast::ObjectNamePart) -> String {
    match ident {
        sqlparser::ast::ObjectNamePart::Identifier(id) => id.value.clone(),
    }
}
