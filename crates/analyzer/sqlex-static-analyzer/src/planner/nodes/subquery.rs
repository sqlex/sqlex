use sqlex_analyzer::AnalyzerError;
use sqlparser::ast::{Query, TableAlias};

use crate::planner::{
    BuildContext,
    plan::{LogicalNode, PlanNode, PlanNodeColumn},
};

#[derive(Debug, Clone)]
pub struct SubqueryNode {
    pub query: Box<dyn PlanNode>,
    pub alias: String,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl SubqueryNode {
    pub fn build(
        ctx: &mut BuildContext,
        subquery: &Query,
        alias: &Option<TableAlias>,
    ) -> Result<Self, AnalyzerError> {
        let effective_alias =
            alias
                .as_ref()
                .map(|a| a.name.value.clone())
                .ok_or(AnalyzerError::AnalysisError(
                    "Subquery must have an alias".to_string(),
                ))?;

        let query = ctx.build_plan(subquery)?;
        let mut output_columns = query.columns().to_vec();
        // Update origin table/alias for subquery columns
        for col in &mut output_columns {
            col.origin_table = Some(effective_alias.clone());
        }
        Ok(Self {
            query,
            alias: effective_alias,
            output_columns,
        })
    }
}

impl LogicalNode for SubqueryNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
