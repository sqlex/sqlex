use crate::{
    diagnostics::{Diagnostic, Phase},
    infer::{Inferencer, expression::ExpressionInference, model::metadata::InferColumn},
};

pub(super) fn infer_slot_expression(
    slot_id: u32,
    input_columns: &[InferColumn],
) -> Result<ExpressionInference, Diagnostic> {
    let Some(column) = input_columns
        .iter()
        .find(|column| column.slot_id.is_some_and(|value| value == slot_id))
    else {
        return Err(Diagnostic::new(
            "I4101",
            Phase::Infer,
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
    ) -> Result<ExpressionInference, Diagnostic> {
        let Some(column) = self.outer_scopes.resolve(depth, slot_id) else {
            let error = if depth == 0 || depth > self.outer_scopes.len() {
                Diagnostic::new(
                    "I4106",
                    Phase::Infer,
                    format!("invalid correlated reference depth {depth} for slot {slot_id}"),
                )
            } else {
                Diagnostic::new(
                    "I4107",
                    Phase::Infer,
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
