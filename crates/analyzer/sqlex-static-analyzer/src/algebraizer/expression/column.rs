use sqlex_analyzer::extension::{ident_ext::IdentExt, object_name_ext::ObjectNameExt};
use sqlex_common::dialect::Dialect;
use sqlparser::ast::{Expr, Value};

use crate::{
    algebraizer::{Algebraizer, model::expression::Expression, scope::RelationBinding},
    diagnostics::{Diagnostic, Phase},
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum ResolvedColumnBinding {
    Local { slot_id: u32 },
    Correlated { depth: usize, slot_id: u32 },
}

impl ResolvedColumnBinding {
    pub(crate) fn into_scalar_expr(self) -> Expression {
        match self {
            Self::Local { slot_id } => Expression::SlotRef(slot_id),
            Self::Correlated { depth, slot_id } => Expression::CorrelatedRef { depth, slot_id },
        }
    }
}

enum QualifiedResolution {
    Found(u32),
    RelationFoundColumnMissing,
    RelationMissing,
}

impl Algebraizer<'_> {
    pub(crate) fn resolve_unqualified_column(
        &self,
        column_name: &str,
    ) -> Result<ResolvedColumnBinding, Diagnostic> {
        if let Some(slot_id) =
            self.resolve_unqualified_in_scope_level(self.relation_scope.current(), column_name)?
        {
            return Ok(ResolvedColumnBinding::Local { slot_id });
        }

        for (index, scope_level) in self.relation_scope.iter_outer().enumerate() {
            if let Some(slot_id) =
                self.resolve_unqualified_in_scope_level(scope_level, column_name)?
            {
                return Ok(ResolvedColumnBinding::Correlated {
                    depth: index + 1,
                    slot_id,
                });
            }
        }

        Err(Diagnostic::new(
            "A3008",
            Phase::Algebraize,
            format!("column not found: {column_name}"),
        ))
    }

    pub(crate) fn resolve_qualified_column(
        &self,
        qualifier: &str,
        column_name: &str,
    ) -> Result<ResolvedColumnBinding, Diagnostic> {
        match self.resolve_qualified_in_scope_level(
            self.relation_scope.current(),
            qualifier,
            column_name,
        )? {
            QualifiedResolution::Found(slot_id) => {
                return Ok(ResolvedColumnBinding::Local { slot_id });
            },
            QualifiedResolution::RelationFoundColumnMissing => {
                return Err(Diagnostic::new(
                    "A3011",
                    Phase::Algebraize,
                    format!("column not found: {qualifier}.{column_name}"),
                ));
            },
            QualifiedResolution::RelationMissing => {},
        }

        let mut relation_found = false;
        for (index, scope_level) in self.relation_scope.iter_outer().enumerate() {
            match self.resolve_qualified_in_scope_level(scope_level, qualifier, column_name)? {
                QualifiedResolution::Found(slot_id) => {
                    return Ok(ResolvedColumnBinding::Correlated {
                        depth: index + 1,
                        slot_id,
                    });
                },
                QualifiedResolution::RelationFoundColumnMissing => {
                    relation_found = true;
                },
                QualifiedResolution::RelationMissing => {},
            }
        }

        if relation_found {
            Err(Diagnostic::new(
                "A3011",
                Phase::Algebraize,
                format!("column not found: {qualifier}.{column_name}"),
            ))
        } else {
            Err(Diagnostic::new(
                "A3010",
                Phase::Algebraize,
                format!("unknown relation reference: {qualifier}"),
            ))
        }
    }

    fn resolve_unqualified_in_scope_level(
        &self,
        scope_level: &[RelationBinding],
        column_name: &str,
    ) -> Result<Option<u32>, Diagnostic> {
        let mut matched_slots = scope_level
            .iter()
            .flat_map(|scope| {
                scope.schema.columns.iter().filter(move |column| {
                    !scope.hidden_unqualified_slot_ids.contains(&column.slot_id)
                })
            })
            .filter(|column| column.name == column_name)
            .map(|column| column.slot_id);

        match (matched_slots.next(), matched_slots.next()) {
            (None, _) => Ok(None),
            (Some(slot_id), None) => Ok(Some(slot_id)),
            (Some(_), Some(_)) => Err(Diagnostic::new(
                "A3009",
                Phase::Algebraize,
                format!("ambiguous column reference: {column_name}"),
            )),
        }
    }

    fn resolve_qualified_in_scope_level(
        &self,
        scope_level: &[RelationBinding],
        qualifier: &str,
        column_name: &str,
    ) -> Result<QualifiedResolution, Diagnostic> {
        let mut matched_scopes = scope_level.iter().filter(|scope| {
            scope
                .qualifier_names
                .iter()
                .any(|visible_name| visible_name == qualifier)
        });

        let first_scope = matched_scopes.next();
        if matched_scopes.next().is_some() {
            return Err(Diagnostic::new(
                "A3010",
                Phase::Algebraize,
                format!("ambiguous relation reference: {qualifier}"),
            ));
        }
        let Some(scope) = first_scope else {
            return Ok(QualifiedResolution::RelationMissing);
        };

        let mut matched_slots = scope
            .schema
            .columns
            .iter()
            .filter(|column| column.name == column_name)
            .map(|column| column.slot_id);
        match (matched_slots.next(), matched_slots.next()) {
            (None, _) => Ok(QualifiedResolution::RelationFoundColumnMissing),
            (Some(slot_id), None) => Ok(QualifiedResolution::Found(slot_id)),
            (Some(_), Some(_)) => Err(Diagnostic::new(
                "A3009",
                Phase::Algebraize,
                format!("ambiguous column reference: {qualifier}.{column_name}"),
            )),
        }
    }

    pub(crate) fn derive_output_name(&self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Identifier(ident) => Ok(ident.to_normalized_string(self.dialect)),
            Expr::CompoundIdentifier(idents) => {
                let last = idents.last().ok_or_else(|| {
                    Diagnostic::new(
                        "A3012",
                        Phase::Algebraize,
                        "empty compound identifier in projection",
                    )
                })?;
                Ok(last.to_normalized_string(self.dialect))
            },
            Expr::Function(function) => {
                let function_name = function.name.to_normalized_string(self.dialect);
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
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use sqlex_common::dialect::Dialect;

    use crate::{
        algebraizer::{
            Algebraizer,
            model::{
                expression::Expression,
                schema::{BoundColumn, ColumnOrigin, OutputSchema},
            },
            scope::RelationBinding,
        },
        catalog::Catalog,
        functions::FunctionRegistry,
    };

    fn make_scope(
        relation_id: u32,
        visible_name: &str,
        columns: &[(u32, &str)],
    ) -> RelationBinding {
        RelationBinding {
            qualifier_names: vec![visible_name.to_string()],
            schema: OutputSchema {
                relation_id,
                columns: columns
                    .iter()
                    .map(|(slot_id, name)| BoundColumn {
                        slot_id: *slot_id,
                        name: (*name).to_string(),
                        table_alias: Some(visible_name.to_string()),
                        data_type: None,
                        nullable: true,
                        origin: ColumnOrigin::Derived,
                    })
                    .collect(),
            },
            hidden_unqualified_slot_ids: HashSet::new(),
        }
    }

    #[test]
    fn resolve_unqualified_prefers_current_scope() {
        let catalog = Catalog::new();
        let functions = FunctionRegistry::new(Dialect::Postgres);
        let mut algebraizer = Algebraizer::new(Dialect::Postgres, &catalog, &functions);
        algebraizer
            .relation_scope
            .set_current(vec![make_scope(2, "outer", &[(2, "id")])]);
        algebraizer.relation_scope.push();
        algebraizer
            .relation_scope
            .set_current(vec![make_scope(1, "cur", &[(1, "id")])]);

        let binding = algebraizer
            .resolve_unqualified_column("id")
            .expect("binding should succeed");
        assert!(matches!(binding.into_scalar_expr(), Expression::SlotRef(1)));
    }

    #[test]
    fn resolve_unqualified_binds_correlated_depth() {
        let catalog = Catalog::new();
        let functions = FunctionRegistry::new(Dialect::Postgres);
        let mut algebraizer = Algebraizer::new(Dialect::Postgres, &catalog, &functions);
        algebraizer
            .relation_scope
            .set_current(vec![make_scope(2, "outer_lv2", &[(20, "id")])]);
        algebraizer.relation_scope.push();
        algebraizer
            .relation_scope
            .set_current(vec![make_scope(3, "outer_lv1", &[(30, "id")])]);
        algebraizer.relation_scope.push();
        algebraizer
            .relation_scope
            .set_current(vec![make_scope(1, "cur", &[(1, "cur_col")])]);

        let binding = algebraizer
            .resolve_unqualified_column("id")
            .expect("binding should succeed");
        assert!(matches!(
            binding.into_scalar_expr(),
            Expression::CorrelatedRef {
                depth: 1,
                slot_id: 30
            }
        ));
    }

    #[test]
    fn resolve_qualified_binds_correlated_depth() {
        let catalog = Catalog::new();
        let functions = FunctionRegistry::new(Dialect::Postgres);
        let mut algebraizer = Algebraizer::new(Dialect::Postgres, &catalog, &functions);
        algebraizer
            .relation_scope
            .set_current(vec![make_scope(2, "t2", &[(20, "id")])]);
        algebraizer.relation_scope.push();
        algebraizer
            .relation_scope
            .set_current(vec![make_scope(3, "t1", &[(30, "id")])]);
        algebraizer.relation_scope.push();
        algebraizer
            .relation_scope
            .set_current(vec![make_scope(1, "cur", &[(1, "cur_col")])]);

        let binding = algebraizer
            .resolve_qualified_column("t1", "id")
            .expect("binding should succeed");
        assert!(matches!(
            binding.into_scalar_expr(),
            Expression::CorrelatedRef {
                depth: 1,
                slot_id: 30
            }
        ));
    }

    #[test]
    fn resolve_unqualified_reports_ambiguous_current_scope() {
        let catalog = Catalog::new();
        let functions = FunctionRegistry::new(Dialect::Postgres);
        let mut algebraizer = Algebraizer::new(Dialect::Postgres, &catalog, &functions);
        algebraizer.relation_scope.set_current(vec![
            make_scope(1, "t1", &[(1, "id")]),
            make_scope(2, "t2", &[(2, "id")]),
        ]);

        let error = algebraizer
            .resolve_unqualified_column("id")
            .expect_err("binding should fail");
        assert_eq!(error.code, "A3009");
    }
}
