use sqlex_analyzer::error::AnalyzerError;

use crate::{
    algebraizer::model::relation::Relation,
    infer::{
        Inferencer, error_code,
        expression::ExpressionInference,
        model::{cardinality::MinRows, metadata::InferColumn},
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_single_column_subquery_expression(
        &mut self,
        subquery: &Relation,
        input_columns: &[InferColumn],
    ) -> Result<ExpressionInference, AnalyzerError> {
        self.outer_scopes.push(input_columns);
        let metadata = self.infer_relation(subquery);
        self.outer_scopes.pop();
        let metadata = metadata?;
        let metadata = self.narrow_int_literals_at_boundary(metadata);

        if metadata.columns.len() != 1 {
            return Err(AnalyzerError::analysis(
                error_code::SUBQUERY_EXPECTS_SINGLE_COLUMN,
                format!(
                    "subquery expression expects exactly one column, got {}",
                    metadata.columns.len()
                ),
            ));
        }

        let column = &metadata.columns[0];
        Ok(ExpressionInference {
            data_type: column.data_type.clone(),
            nullable: column.nullable || !matches!(metadata.cardinality.min(), MinRows::One),
            int_literal_info: None,
        })
    }
}
