use std::collections::HashSet;

use sqlparser::ast::Select;

use crate::{
    algebraizer::{
        Algebraizer,
        context::{BuildContext, RelationScope},
        model::{
            relation::{Relation, ValuesNode},
            schema::OutputSchema,
        },
    },
    catalog::Catalog,
    diagnostics::{Diagnostic, Phase},
    functions::FunctionRegistry,
};

impl Algebraizer {
    pub(crate) fn build_from(
        &self,
        select: &Select,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &mut BuildContext,
    ) -> Result<Relation, Diagnostic> {
        if select.from.is_empty() {
            let schema = OutputSchema {
                relation_id: context.allocate_relation_id(),
                columns: Vec::new(),
            };
            context.relation_scopes = vec![RelationScope {
                visible_names: vec![],
                schema: schema.clone(),
                hidden_unqualified_slots: HashSet::new(),
            }];
            return Ok(Relation::Values(ValuesNode { schema }));
        }

        if select.from.len() != 1 {
            return Err(Diagnostic::new(
                "A3071",
                Phase::Algebraize,
                "multiple FROM items are not supported in this iteration",
            ));
        }

        let from_item = &select.from[0];
        let (mut relation, left_scope) =
            self.build_table_factor(&from_item.relation, catalog, functions, context)?;
        let mut scopes = vec![left_scope];
        context.relation_scopes = scopes.clone();

        for join in &from_item.joins {
            relation = self.build_join(relation, &mut scopes, join, catalog, functions, context)?;
        }

        context.relation_scopes = scopes;
        Ok(relation)
    }
}
