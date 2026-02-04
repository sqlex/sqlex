use sqlex_analyzer::AnalyzerError;

use crate::planner::{
    BuildContext,
    plan::{LogicalNode, PlanNodeColumn},
};

#[derive(Debug, Clone)]
pub struct CTERefNode {
    pub name: String,
    pub alias: Option<String>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl CTERefNode {
    pub fn build(
        ctx: &BuildContext,
        name: String,
        alias: Option<String>,
    ) -> Result<Option<Self>, AnalyzerError> {
        let cte = match ctx.cte_scope.get(&name) {
            Some(cte) => cte,
            None => return Ok(None),
        };

        let mut output_columns = cte
            .columns
            .iter()
            .map(|c| PlanNodeColumn {
                name: c.name.clone(),
                data_type: c.data_type.clone(),
                nullability: c.nullable,
                origin_table: c.source_alias.clone(),
                origin_column: None,
            })
            .collect();

        // Update origin table if alias is provided, or just to reflect this CTE usage
        let origin_name = alias.as_ref().unwrap_or(&name).clone();
        for col in &mut output_columns {
            col.origin_table = Some(origin_name.clone());
        }

        Ok(Some(Self {
            name,
            alias,
            output_columns,
        }))
    }
}

impl LogicalNode for CTERefNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
