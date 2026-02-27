use std::collections::HashSet;

use sqlex_analyzer::error::AnalyzerError;
use sqlparser::ast::Select;

use crate::algebraizer::{
    Algebraizer, error_code,
    model::{
        relation::{Relation, ValuesNode},
        schema::OutputSchema,
    },
    scope::RelationBinding,
};

impl Algebraizer<'_> {
    pub(crate) fn build_from_relation(
        &mut self,
        select: &Select,
    ) -> Result<Relation, AnalyzerError> {
        if select.from.is_empty() {
            let schema = OutputSchema {
                relation_id: self.allocate_relation_id(),
                columns: Vec::new(),
            };
            self.relation_scope.set_current(vec![RelationBinding {
                qualifier_names: vec![],
                schema: schema.clone(),
                hidden_unqualified_slot_ids: HashSet::new(),
            }]);
            return Ok(Relation::Values(ValuesNode { schema }));
        }

        if select.from.len() != 1 {
            return Err(AnalyzerError::analysis(
                error_code::MULTIPLE_FROM_ITEMS_UNSUPPORTED,
                "multiple FROM items are not supported in this iteration",
            ));
        }

        let from_item = &select.from[0];
        let (mut relation, left_scope) = self.build_table_factor_relation(&from_item.relation)?;
        let mut scopes = vec![left_scope];
        self.relation_scope.set_current(scopes.clone());

        for join in &from_item.joins {
            relation = self.build_join_relation(relation, &mut scopes, join)?;
        }

        self.relation_scope.set_current(scopes);
        Ok(relation)
    }
}
