use sqlex_analyzer::error::AnalyzerError;

use crate::infer::{
    Inferencer, error_code, expression::ExpressionInference, model::metadata::InferColumn,
};

pub(super) fn infer_slot_expression(
    slot_id: u32,
    input_columns: &[InferColumn],
) -> Result<ExpressionInference, AnalyzerError> {
    let Some(column) = input_columns
        .iter()
        .find(|column| column.slot_id.is_some_and(|value| value == slot_id))
    else {
        return Err(AnalyzerError::analysis(
            error_code::SLOT_REFERENCE_UNKNOWN,
            format!("unknown slot reference: {slot_id}"),
        ));
    };

    Ok(ExpressionInference {
        data_type: column.data_type.clone(),
        nullable: column.nullable,
        int_literal_info: None,
    })
}

impl Inferencer<'_> {
    pub(super) fn infer_correlated_slot_expression(
        &self,
        depth: usize,
        slot_id: u32,
    ) -> Result<ExpressionInference, AnalyzerError> {
        let Some(column) = self.outer_scopes.resolve(depth, slot_id) else {
            let error = if depth == 0 || depth > self.outer_scopes.len() {
                AnalyzerError::analysis(
                    error_code::CORRELATED_REFERENCE_DEPTH_INVALID,
                    format!("invalid correlated reference depth {depth} for slot {slot_id}"),
                )
            } else {
                AnalyzerError::analysis(
                    error_code::CORRELATED_SLOT_REFERENCE_UNKNOWN,
                    format!("unknown correlated slot reference: slot {slot_id}, depth {depth}"),
                )
            };
            return Err(error);
        };

        Ok(ExpressionInference {
            data_type: column.data_type.clone(),
            nullable: column.nullable,
            int_literal_info: None,
        })
    }
}
