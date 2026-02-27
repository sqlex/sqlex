use async_trait::async_trait;
use sqlex_analyzer::{Analyzer, Result};
use sqlex_common::types::{ColumnInfo, ResultSet, Table};

use crate::{StaticAnalyzer, algebraizer::Algebraizer, infer::Inferencer};

#[async_trait]
impl Analyzer for StaticAnalyzer {
    async fn execute(&mut self, sql: &str) -> Result<()> {
        let statement = self.parse_statement(sql)?;
        self.catalog.execute(self.dialect, &statement)?;

        Ok(())
    }

    async fn analyze(&self, sql: &str) -> Result<ResultSet> {
        let statement = self.parse_statement(sql)?;
        let algebraizer = Algebraizer::new(self.dialect, &self.catalog, &self.functions);
        let relation = algebraizer.build(&statement)?;
        let inferencer = Inferencer::new(self.dialect, &self.catalog, &self.functions);
        let metadata = inferencer.infer(&relation)?;

        Ok(metadata.to_result_set())
    }

    async fn get_all_tables(&self) -> Result<Vec<Table>> {
        Ok(self
            .catalog
            .get_all_tables()
            .into_iter()
            .map(|table| Table {
                name: table.name,
                columns: table
                    .columns
                    .into_iter()
                    .map(|column| ColumnInfo {
                        name: column.name,
                        data_type: column.data_type,
                        nullability: column.nullable,
                    })
                    .collect(),
            })
            .collect())
    }
}
