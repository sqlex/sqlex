use std::collections::HashSet;

use sqlparser::ast::Select;

use crate::{
    algebraizer::{
        Algebraizer,
        model::{
            relation::{Relation, ValuesNode},
            schema::OutputSchema,
        },
        scope::RelationBinding,
    },
    diagnostics::{Diagnostic, Phase},
};

impl Algebraizer<'_> {
    pub(crate) fn build_from(&mut self, select: &Select) -> Result<Relation, Diagnostic> {
        if select.from.is_empty() {
            let schema = OutputSchema {
                relation_id: self.allocate_relation_id(),
                columns: Vec::new(),
            };
            self.set_current_relation_bindings(vec![RelationBinding {
                qualifier_names: vec![],
                schema: schema.clone(),
                hidden_unqualified_slot_ids: HashSet::new(),
            }]);
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
        let (mut relation, left_scope) = self.build_table_factor(&from_item.relation)?;
        let mut scopes = vec![left_scope];
        self.set_current_relation_bindings(scopes.clone());

        for join in &from_item.joins {
            relation = self.build_join(relation, &mut scopes, join)?;
        }

        self.set_current_relation_bindings(scopes);
        Ok(relation)
    }
}
