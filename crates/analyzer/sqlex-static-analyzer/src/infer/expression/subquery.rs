use crate::{
    algebraizer::model::relation::Relation,
    diagnostics::{Diagnostic, Phase},
    infer::{
        Inferencer,
        expression::ExpressionInference,
        model::{cardinality::MinRows, metadata::InferColumn},
    },
};

impl Inferencer<'_> {
    pub(super) fn infer_single_column_subquery_expression(
        &self,
        subquery: &Relation,
        input_columns: &[InferColumn],
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<ExpressionInference, Diagnostic> {
        let mut subquery_outer_scopes = outer_scopes.to_vec();
        subquery_outer_scopes.push(input_columns.to_vec());
        let metadata = self.infer_relation_with_outer_scopes(subquery, &subquery_outer_scopes)?;
        if metadata.columns.len() != 1 {
            return Err(Diagnostic::new(
                "I4105",
                Phase::Infer,
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
        })
    }
}
