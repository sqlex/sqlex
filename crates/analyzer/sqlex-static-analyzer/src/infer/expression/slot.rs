use crate::{
    diagnostics::{Diagnostic, Phase},
    infer::{expression::ExpressionInference, model::metadata::InferColumn},
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

pub(super) fn infer_correlated_slot_expression(
    depth: usize,
    slot_id: u32,
    outer_scopes: &[Vec<InferColumn>],
) -> Result<ExpressionInference, Diagnostic> {
    if depth == 0 || depth > outer_scopes.len() {
        return Err(Diagnostic::new(
            "I4106",
            Phase::Infer,
            format!("invalid correlated reference depth {depth} for slot {slot_id}"),
        ));
    }

    let scope_index = outer_scopes.len() - depth;
    let Some(column) = outer_scopes[scope_index]
        .iter()
        .find(|column| column.slot_id.is_some_and(|value| value == slot_id))
    else {
        return Err(Diagnostic::new(
            "I4107",
            Phase::Infer,
            format!("unknown correlated slot reference: slot {slot_id}, depth {depth}"),
        ));
    };

    Ok(ExpressionInference {
        data_type: column.data_type.clone(),
        nullable: column.nullable,
        int_literal_info: None,
    })
}
