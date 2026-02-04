use sqlex_analyzer::AnalyzerError;

use crate::{
    planner::{
        BuildContext,
        plan::{LogicalNode, PlanNodeColumn},
    },
};

#[derive(Debug, Clone)]
pub struct TableScanNode {
    pub table: String,
    pub alias: Option<String>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl TableScanNode {
    pub fn build(
        ctx: &BuildContext,
        table_name: String,
        alias: Option<String>,
    ) -> Result<Self, AnalyzerError> {
        let effective_alias = alias.as_deref().unwrap_or(&table_name).to_string();

        let table_def = ctx.schema.tables.get(&table_name).ok_or_else(|| {
            AnalyzerError::AnalysisError(format!("Table {} not found", table_name))
        })?;

        let output_columns = table_def
            .columns
            .iter()
            .map(|c| PlanNodeColumn {
                name: c.name.clone(),
                data_type: c.data_type.clone(),
                nullability: c.nullable,
                origin_table: Some(effective_alias.clone()),
                origin_column: Some(c.name.clone()),
            })
            .collect();

        Ok(Self {
            table: table_name,
            alias,
            output_columns,
        })
    }
}

impl LogicalNode for TableScanNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
