use std::collections::{HashMap, HashSet};

use sqlex_analyzer::extension::DataTypeExt;
use sqlex_common::{dialect::Dialect, types::DataType};

use crate::{
    algebra::{
        expr::{
            AggregationNode, AliasNode, JoinKind, JoinNode, LimitNode, ProjectionNode, RelExpr,
            SetOp, SetOpNode, SortNode, WindowNode,
        },
        scalar::{ColumnOrigin as BoundColumnOrigin, OutputSchema},
    },
    catalog::model::Catalog,
    diagnostics::{Diagnostic, Phase},
    functions::registry::FunctionRegistry,
    infer::{
        cardinality::{CardInterval, MaxRows, MinRows},
        metadata::{ColumnOrigin, InferColumn, InferMetadata, ResolvedKey},
        scalar_infer::infer_scalar,
    },
};

pub(crate) fn infer_operator(
    expr: &RelExpr,
    catalog: &Catalog,
    dialect: Dialect,
    functions: &FunctionRegistry,
) -> Result<InferMetadata, Diagnostic> {
    infer_operator_with_outer_scopes(expr, catalog, dialect, functions, &[])
}

pub(crate) fn infer_operator_with_outer_scopes(
    expr: &RelExpr,
    catalog: &Catalog,
    dialect: Dialect,
    functions: &FunctionRegistry,
    outer_scopes: &[Vec<InferColumn>],
) -> Result<InferMetadata, Diagnostic> {
    match expr {
        RelExpr::Scan(node) => infer_scan(node, catalog),
        RelExpr::Values(_) => Ok(InferMetadata {
            columns: Vec::new(),
            cardinality: CardInterval::exactly_one(),
            keys: Vec::new(),
        }),
        RelExpr::Selection(node) => {
            let mut child = infer_operator_with_outer_scopes(
                &node.input,
                catalog,
                dialect,
                functions,
                outer_scopes,
            )?;
            let _ = infer_scalar(
                &node.condition,
                &child.columns,
                catalog,
                dialect,
                functions,
                outer_scopes,
            )?;
            if always_false_condition(&node.condition) {
                child.cardinality = CardInterval::at_most_one();
            }
            if selection_is_at_most_one(&node.condition, &child.keys, &child.columns) {
                child.cardinality = child.cardinality.constrain_at_most_one();
            }
            Ok(child)
        },
        RelExpr::Aggregation(node) => {
            infer_aggregation(node, catalog, dialect, functions, outer_scopes)
        },
        RelExpr::Window(node) => infer_window(node, catalog, dialect, functions, outer_scopes),
        RelExpr::Projection(node) => {
            infer_projection(node, catalog, dialect, functions, outer_scopes)
        },
        RelExpr::Join(node) => infer_join(node, catalog, dialect, functions, outer_scopes),
        RelExpr::Distinct(node) => infer_distinct(node, catalog, dialect, functions, outer_scopes),
        RelExpr::Sort(node) => infer_sort(node, catalog, dialect, functions, outer_scopes),
        RelExpr::Limit(node) => infer_limit(node, catalog, dialect, functions, outer_scopes),
        RelExpr::Alias(node) => infer_alias(node, catalog, dialect, functions, outer_scopes),
        RelExpr::SetOperation(node) => {
            infer_set_operation(node, catalog, dialect, functions, outer_scopes)
        },
        RelExpr::PlaceholderQuery => Err(Diagnostic::todo(
            Phase::Infer,
            "placeholder query inference",
        )),
    }
}

fn infer_scan(
    node: &crate::algebra::expr::ScanNode,
    catalog: &Catalog,
) -> Result<InferMetadata, Diagnostic> {
    let _ = &node.table;
    let mut columns = Vec::with_capacity(node.schema.columns.len());

    for column in &node.schema.columns {
        let data_type = column
            .data_type
            .clone()
            .unwrap_or_else(|| DataType::Custom("unknown".to_string()));
        let origin = match &column.origin {
            BoundColumnOrigin::Base { table, column } => ColumnOrigin::Base {
                table: table.clone(),
                column: column.clone(),
            },
            BoundColumnOrigin::Derived => ColumnOrigin::Derived,
        };

        columns.push(InferColumn {
            slot_id: Some(column.slot_id),
            name: column.name.clone(),
            data_type,
            nullable: column.nullable,
            origin,
        });
    }

    let keys = resolve_scan_keys(node, catalog);

    Ok(InferMetadata {
        columns,
        cardinality: if node.table.starts_with("__recursive_cte__") {
            CardInterval::one_or_more()
        } else {
            CardInterval::zero_or_more()
        },
        keys,
    })
}

fn infer_aggregation(
    node: &AggregationNode,
    catalog: &Catalog,
    dialect: Dialect,
    functions: &FunctionRegistry,
    outer_scopes: &[Vec<InferColumn>],
) -> Result<InferMetadata, Diagnostic> {
    let child =
        infer_operator_with_outer_scopes(&node.input, catalog, dialect, functions, outer_scopes)?;
    for projection in &node.group_by {
        let _ = infer_scalar(
            &projection.expr,
            &child.columns,
            catalog,
            dialect,
            functions,
            outer_scopes,
        )?;
    }
    for projection in &node.aggregates {
        let _ = infer_scalar(
            &projection.expr,
            &child.columns,
            catalog,
            dialect,
            functions,
            outer_scopes,
        )?;
    }

    let columns = align_columns_to_schema(&child.columns, &node.schema);
    let cardinality = if node.group_by.is_empty() && !node.aggregates.is_empty() {
        CardInterval::exactly_one()
    } else {
        child.cardinality
    };

    Ok(InferMetadata {
        columns,
        cardinality,
        keys: Vec::new(),
    })
}

fn infer_window(
    node: &WindowNode,
    catalog: &Catalog,
    dialect: Dialect,
    functions: &FunctionRegistry,
    outer_scopes: &[Vec<InferColumn>],
) -> Result<InferMetadata, Diagnostic> {
    let child =
        infer_operator_with_outer_scopes(&node.input, catalog, dialect, functions, outer_scopes)?;
    for projection in &node.window_exprs {
        let _ = infer_scalar(
            &projection.expr,
            &child.columns,
            catalog,
            dialect,
            functions,
            outer_scopes,
        )?;
    }

    Ok(InferMetadata {
        columns: align_columns_to_schema(&child.columns, &node.schema),
        cardinality: child.cardinality,
        keys: child.keys,
    })
}

fn infer_projection(
    node: &ProjectionNode,
    catalog: &Catalog,
    dialect: Dialect,
    functions: &FunctionRegistry,
    outer_scopes: &[Vec<InferColumn>],
) -> Result<InferMetadata, Diagnostic> {
    let child =
        infer_operator_with_outer_scopes(&node.input, catalog, dialect, functions, outer_scopes)?;

    let mut columns = Vec::with_capacity(node.columns.len());
    let mut slot_mapping = HashMap::new();
    for (index, projection_column) in node.columns.iter().enumerate() {
        let scalar = infer_scalar(
            &projection_column.expr,
            &child.columns,
            catalog,
            dialect,
            functions,
            outer_scopes,
        )?;
        let output_slot_id = node.schema.columns.get(index).map(|column| column.slot_id);
        let output_name = projection_column.alias.clone().ok_or_else(|| {
            Diagnostic::new(
                "I4201",
                Phase::Infer,
                "projection column alias was not assigned during planning",
            )
        })?;
        if let (
            crate::algebra::scalar::BoundScalarExpr::SlotRef(input_slot_id),
            Some(output_slot_id),
        ) = (&projection_column.expr, output_slot_id)
        {
            slot_mapping.insert(*input_slot_id, output_slot_id);
        }

        columns.push(InferColumn {
            slot_id: output_slot_id,
            name: output_name,
            data_type: scalar.data_type,
            nullable: scalar.nullable,
            origin: ColumnOrigin::Derived,
        });
    }

    Ok(InferMetadata {
        columns,
        cardinality: child.cardinality,
        keys: child.remap_keys(&slot_mapping),
    })
}

fn infer_join(
    node: &JoinNode,
    catalog: &Catalog,
    dialect: Dialect,
    functions: &FunctionRegistry,
    outer_scopes: &[Vec<InferColumn>],
) -> Result<InferMetadata, Diagnostic> {
    let left =
        infer_operator_with_outer_scopes(&node.left, catalog, dialect, functions, outer_scopes)?;
    let right =
        infer_operator_with_outer_scopes(&node.right, catalog, dialect, functions, outer_scopes)?;

    let mut left_columns = left.columns;
    let mut right_columns = right.columns;
    match node.kind {
        JoinKind::Left => {
            for column in &mut right_columns {
                column.nullable = true;
            }
        },
        JoinKind::Right => {
            for column in &mut left_columns {
                column.nullable = true;
            }
        },
        JoinKind::Full => {
            for column in &mut left_columns {
                column.nullable = true;
            }
            for column in &mut right_columns {
                column.nullable = true;
            }
        },
        JoinKind::Inner | JoinKind::Cross => {},
    }

    let mut columns_by_slot = HashMap::new();
    for column in left_columns.into_iter().chain(right_columns) {
        if let Some(slot_id) = column.slot_id {
            columns_by_slot.insert(slot_id, column);
        }
    }

    let mut columns = Vec::with_capacity(node.schema.columns.len());
    for schema_column in &node.schema.columns {
        if let Some(source_column) = columns_by_slot.get(&schema_column.slot_id) {
            columns.push(InferColumn {
                slot_id: Some(schema_column.slot_id),
                name: schema_column.name.clone(),
                data_type: source_column.data_type.clone(),
                nullable: source_column.nullable,
                origin: source_column.origin.clone(),
            });
            continue;
        }

        let data_type = schema_column
            .data_type
            .clone()
            .unwrap_or_else(|| DataType::Custom("unknown".to_string()));
        let origin = match &schema_column.origin {
            BoundColumnOrigin::Base { table, column } => ColumnOrigin::Base {
                table: table.clone(),
                column: column.clone(),
            },
            BoundColumnOrigin::Derived => ColumnOrigin::Derived,
        };

        columns.push(InferColumn {
            slot_id: Some(schema_column.slot_id),
            name: schema_column.name.clone(),
            data_type,
            nullable: schema_column.nullable,
            origin,
        });
    }

    let cardinality = match node.kind {
        JoinKind::Left | JoinKind::Right | JoinKind::Full | JoinKind::Inner => {
            CardInterval::zero_or_more()
        },
        JoinKind::Cross => CardInterval::zero_or_more(),
    };

    Ok(InferMetadata {
        columns,
        cardinality,
        keys: Vec::new(),
    })
}

fn infer_distinct(
    node: &crate::algebra::expr::DistinctNode,
    catalog: &Catalog,
    dialect: Dialect,
    functions: &FunctionRegistry,
    outer_scopes: &[Vec<InferColumn>],
) -> Result<InferMetadata, Diagnostic> {
    let mut child =
        infer_operator_with_outer_scopes(&node.input, catalog, dialect, functions, outer_scopes)?;
    let output_columns = align_columns_to_schema(&child.columns, &node.schema);
    child.keys = slots_key(output_columns.iter().filter_map(|column| column.slot_id));

    Ok(InferMetadata {
        columns: output_columns,
        cardinality: child.cardinality,
        keys: child.keys,
    })
}

fn infer_sort(
    node: &SortNode,
    catalog: &Catalog,
    dialect: Dialect,
    functions: &FunctionRegistry,
    outer_scopes: &[Vec<InferColumn>],
) -> Result<InferMetadata, Diagnostic> {
    let child =
        infer_operator_with_outer_scopes(&node.input, catalog, dialect, functions, outer_scopes)?;
    for key in &node.keys {
        let _ = infer_scalar(
            &key.expr,
            &child.columns,
            catalog,
            dialect,
            functions,
            outer_scopes,
        )?;
    }
    Ok(InferMetadata {
        columns: align_columns_to_schema(&child.columns, &node.schema),
        cardinality: child.cardinality,
        keys: child.keys,
    })
}

fn infer_alias(
    node: &AliasNode,
    catalog: &Catalog,
    dialect: Dialect,
    functions: &FunctionRegistry,
    outer_scopes: &[Vec<InferColumn>],
) -> Result<InferMetadata, Diagnostic> {
    let child =
        infer_operator_with_outer_scopes(&node.input, catalog, dialect, functions, outer_scopes)?;
    Ok(InferMetadata {
        columns: align_columns_to_schema(&child.columns, &node.schema),
        cardinality: child.cardinality,
        keys: child.keys,
    })
}

fn infer_limit(
    node: &LimitNode,
    catalog: &Catalog,
    dialect: Dialect,
    functions: &FunctionRegistry,
    outer_scopes: &[Vec<InferColumn>],
) -> Result<InferMetadata, Diagnostic> {
    let mut child =
        infer_operator_with_outer_scopes(&node.input, catalog, dialect, functions, outer_scopes)?;

    if node.offset.is_some() {
        child.cardinality = child.cardinality.drop_lower_bound();
    }
    if node.limit.is_some_and(|value| value <= 1) {
        child.cardinality = child.cardinality.constrain_at_most_one();
    }

    Ok(child)
}

fn infer_set_operation(
    node: &SetOpNode,
    catalog: &Catalog,
    dialect: Dialect,
    functions: &FunctionRegistry,
    outer_scopes: &[Vec<InferColumn>],
) -> Result<InferMetadata, Diagnostic> {
    let left =
        infer_operator_with_outer_scopes(&node.left, catalog, dialect, functions, outer_scopes)?;
    let right =
        infer_operator_with_outer_scopes(&node.right, catalog, dialect, functions, outer_scopes)?;
    if left.columns.len() != right.columns.len() {
        return Err(Diagnostic::new(
            "I4202",
            Phase::Infer,
            format!(
                "set operation column count mismatch: left {}, right {}",
                left.columns.len(),
                right.columns.len()
            ),
        ));
    }

    let mut columns = Vec::with_capacity(left.columns.len());
    for index in 0..left.columns.len() {
        let left_column = &left.columns[index];
        let right_column = &right.columns[index];
        let output_slot_id = node.schema.columns.get(index).map(|column| column.slot_id);
        let output_name = node
            .schema
            .columns
            .get(index)
            .map(|column| column.name.clone())
            .unwrap_or_else(|| left_column.name.clone());
        let data_type = DataType::common_type(
            dialect,
            &[
                left_column.data_type.clone(),
                right_column.data_type.clone(),
            ],
        )
        .unwrap_or_else(|| left_column.data_type.clone());

        columns.push(InferColumn {
            slot_id: output_slot_id,
            name: output_name,
            data_type,
            nullable: left_column.nullable || right_column.nullable,
            origin: ColumnOrigin::Derived,
        });
    }

    let cardinality = infer_set_operation_cardinality(
        node.op.clone(),
        node.all,
        left.cardinality,
        right.cardinality,
    )?;

    Ok(InferMetadata {
        columns,
        cardinality,
        keys: Vec::new(),
    })
}

fn infer_set_operation_cardinality(
    op: SetOp,
    all: bool,
    left: CardInterval,
    right: CardInterval,
) -> Result<CardInterval, Diagnostic> {
    match op {
        SetOp::Union => {
            let min = if matches!(left.min(), MinRows::One) || matches!(right.min(), MinRows::One) {
                MinRows::One
            } else {
                MinRows::Zero
            };
            let max = match (left.max(), right.max()) {
                (MaxRows::Zero, MaxRows::Zero) => MaxRows::Zero,
                _ => MaxRows::Many,
            };
            if all {
                CardInterval::try_new(min, max, "infer_set_operation_cardinality::union_all")
            } else {
                CardInterval::try_new(min, max, "infer_set_operation_cardinality::union")
            }
        },
        SetOp::Intersect => CardInterval::try_new(
            if matches!(left.min(), MinRows::One) && matches!(right.min(), MinRows::One) {
                MinRows::One
            } else {
                MinRows::Zero
            },
            min_max_rows(left.max(), right.max()),
            "infer_set_operation_cardinality::intersect",
        ),
        SetOp::Except => Ok(left),
    }
}

fn min_max_rows(left: MaxRows, right: MaxRows) -> MaxRows {
    match (left, right) {
        (MaxRows::Zero, _) | (_, MaxRows::Zero) => MaxRows::Zero,
        (MaxRows::One, MaxRows::One) => MaxRows::One,
        _ => MaxRows::Many,
    }
}

fn resolve_scan_keys(node: &crate::algebra::expr::ScanNode, catalog: &Catalog) -> Vec<ResolvedKey> {
    let Some(table) = catalog.table(&node.table) else {
        return Vec::new();
    };

    let slot_by_column: HashMap<String, u32> = node
        .schema
        .columns
        .iter()
        .filter_map(|column| match &column.origin {
            BoundColumnOrigin::Base {
                table,
                column: name,
            } if table == &node.table => Some((name.clone(), column.slot_id)),
            _ => None,
        })
        .collect();

    let mut unique = HashSet::new();
    let mut keys = Vec::new();
    if let Some(primary_key) = &table.primary_key {
        if let Some(key) = key_from_column_names(&primary_key.columns, &slot_by_column) {
            let identity = key.slot_ids.clone();
            if unique.insert(identity) {
                keys.push(key);
            }
        }
    }
    for unique_key in &table.unique_keys {
        if let Some(key) = key_from_column_names(&unique_key.columns, &slot_by_column) {
            let identity = key.slot_ids.clone();
            if unique.insert(identity) {
                keys.push(key);
            }
        }
    }
    keys
}

fn key_from_column_names(
    column_names: &[String],
    slot_by_column: &HashMap<String, u32>,
) -> Option<ResolvedKey> {
    let mut slots = Vec::with_capacity(column_names.len());
    for column_name in column_names {
        let slot_id = slot_by_column.get(column_name)?;
        slots.push(*slot_id);
    }
    ResolvedKey::from_slots(slots)
}

fn slots_key(slots: impl IntoIterator<Item = u32>) -> Vec<ResolvedKey> {
    let slots: Vec<u32> = slots.into_iter().collect();
    ResolvedKey::from_slots(slots).map_or_else(Vec::new, |key| vec![key])
}

fn align_columns_to_schema(
    child_columns: &[InferColumn],
    schema: &OutputSchema,
) -> Vec<InferColumn> {
    if child_columns.len() != schema.columns.len() {
        return child_columns.to_vec();
    }

    child_columns
        .iter()
        .zip(schema.columns.iter())
        .map(|(column, schema_column)| InferColumn {
            slot_id: Some(schema_column.slot_id),
            name: schema_column.name.clone(),
            data_type: column.data_type.clone(),
            nullable: column.nullable,
            origin: column.origin.clone(),
        })
        .collect()
}

fn always_false_condition(condition: &crate::algebra::scalar::BoundScalarExpr) -> bool {
    match condition {
        crate::algebra::scalar::BoundScalarExpr::Literal(
            crate::algebra::scalar::BoundLiteral::Bool(value),
        ) => !*value,
        crate::algebra::scalar::BoundScalarExpr::BinaryOp { left, op, right } => match op {
            crate::algebra::scalar::BoundBinaryOp::Eq => {
                literal_comparison_false(left, right, true)
            },
            crate::algebra::scalar::BoundBinaryOp::NotEq => {
                literal_comparison_false(left, right, false)
            },
            _ => false,
        },
        _ => false,
    }
}

fn literal_comparison_false(
    left: &crate::algebra::scalar::BoundScalarExpr,
    right: &crate::algebra::scalar::BoundScalarExpr,
    is_eq: bool,
) -> bool {
    let crate::algebra::scalar::BoundScalarExpr::Literal(left_literal) = left else {
        return false;
    };
    let crate::algebra::scalar::BoundScalarExpr::Literal(right_literal) = right else {
        return false;
    };
    let Some(literals_equal) = literal_equal(left_literal, right_literal) else {
        return false;
    };
    if is_eq {
        !literals_equal
    } else {
        literals_equal
    }
}

fn literal_equal(
    left: &crate::algebra::scalar::BoundLiteral,
    right: &crate::algebra::scalar::BoundLiteral,
) -> Option<bool> {
    match (left, right) {
        (
            crate::algebra::scalar::BoundLiteral::Null,
            crate::algebra::scalar::BoundLiteral::Null,
        ) => Some(true),
        (
            crate::algebra::scalar::BoundLiteral::Bool(left_value),
            crate::algebra::scalar::BoundLiteral::Bool(right_value),
        ) => Some(left_value == right_value),
        (
            crate::algebra::scalar::BoundLiteral::Int {
                value: left_value, ..
            },
            crate::algebra::scalar::BoundLiteral::Int {
                value: right_value, ..
            },
        ) => Some(left_value == right_value),
        (
            crate::algebra::scalar::BoundLiteral::Float(left_value),
            crate::algebra::scalar::BoundLiteral::Float(right_value),
        ) => Some(left_value.to_bits() == right_value.to_bits()),
        (
            crate::algebra::scalar::BoundLiteral::String(left_value),
            crate::algebra::scalar::BoundLiteral::String(right_value),
        ) => Some(left_value == right_value),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SingleValueConstraint {
    EqLike,
    IsNull,
}

fn selection_is_at_most_one(
    condition: &crate::algebra::scalar::BoundScalarExpr,
    input_keys: &[ResolvedKey],
    input_columns: &[InferColumn],
) -> bool {
    let mut constraints = HashMap::<u32, SingleValueConstraint>::new();
    if !collect_single_value_constraints(condition, &mut constraints) {
        return false;
    }

    input_keys
        .iter()
        .any(|key| key_satisfied(key, &constraints, input_columns))
}

fn collect_single_value_constraints(
    expr: &crate::algebra::scalar::BoundScalarExpr,
    constraints: &mut HashMap<u32, SingleValueConstraint>,
) -> bool {
    match expr {
        crate::algebra::scalar::BoundScalarExpr::BinaryOp { left, op, right } => match op {
            crate::algebra::scalar::BoundBinaryOp::And => {
                collect_single_value_constraints(left, constraints)
                    && collect_single_value_constraints(right, constraints)
            },
            crate::algebra::scalar::BoundBinaryOp::Or => false,
            crate::algebra::scalar::BoundBinaryOp::Eq => {
                if let Some(slot_id) = slot_id_equals_single_value(left, right) {
                    set_constraint(constraints, slot_id, SingleValueConstraint::EqLike);
                } else if let Some(slot_id) = slot_id_equals_single_value(right, left) {
                    set_constraint(constraints, slot_id, SingleValueConstraint::EqLike);
                }
                true
            },
            _ => true,
        },
        crate::algebra::scalar::BoundScalarExpr::InList {
            expr,
            list,
            negated,
        } => {
            if !*negated && list.len() == 1 {
                if let crate::algebra::scalar::BoundScalarExpr::SlotRef(slot_id) = expr.as_ref() {
                    set_constraint(constraints, *slot_id, SingleValueConstraint::EqLike);
                }
            }
            true
        },
        crate::algebra::scalar::BoundScalarExpr::IsNull { expr, negated } => {
            if !*negated {
                if let crate::algebra::scalar::BoundScalarExpr::SlotRef(slot_id) = expr.as_ref() {
                    set_constraint(constraints, *slot_id, SingleValueConstraint::IsNull);
                }
            }
            true
        },
        _ => true,
    }
}

fn slot_id_equals_single_value(
    left: &crate::algebra::scalar::BoundScalarExpr,
    right: &crate::algebra::scalar::BoundScalarExpr,
) -> Option<u32> {
    let crate::algebra::scalar::BoundScalarExpr::SlotRef(slot_id) = left else {
        return None;
    };
    if is_single_value_expr(right) {
        Some(*slot_id)
    } else {
        None
    }
}

fn is_single_value_expr(expr: &crate::algebra::scalar::BoundScalarExpr) -> bool {
    match expr {
        crate::algebra::scalar::BoundScalarExpr::Literal(_) => true,
        crate::algebra::scalar::BoundScalarExpr::Cast { expr, .. } => is_single_value_expr(expr),
        _ => false,
    }
}

fn set_constraint(
    constraints: &mut HashMap<u32, SingleValueConstraint>,
    slot_id: u32,
    constraint: SingleValueConstraint,
) {
    match constraints.get(&slot_id) {
        Some(SingleValueConstraint::EqLike) => {},
        Some(SingleValueConstraint::IsNull) if constraint == SingleValueConstraint::EqLike => {
            constraints.insert(slot_id, SingleValueConstraint::EqLike);
        },
        None => {
            constraints.insert(slot_id, constraint);
        },
        _ => {},
    }
}

fn key_satisfied(
    key: &ResolvedKey,
    constraints: &HashMap<u32, SingleValueConstraint>,
    input_columns: &[InferColumn],
) -> bool {
    if key.slot_ids.is_empty() {
        return false;
    }

    for slot_id in &key.slot_ids {
        let Some(constraint) = constraints.get(slot_id) else {
            return false;
        };
        if *constraint == SingleValueConstraint::EqLike {
            continue;
        }
        let Some(column) = input_columns
            .iter()
            .find(|column| column.slot_id.is_some_and(|value| value == *slot_id))
        else {
            return false;
        };
        if column.nullable {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use sqlex_common::{dialect::Dialect, types::Cardinality};

    use crate::{
        algebra::{
            expr::{ProjectionNode, RelExpr, ScanNode, SelectionNode},
            scalar::{
                BoundBinaryOp, BoundColumn, BoundLiteral, BoundScalarExpr, ColumnOrigin,
                OutputSchema, ProjectionColumn, Visibility,
            },
        },
        catalog::model::{Catalog, ColumnSchema, KeyConstraint, TableSchema},
        functions::registry::FunctionRegistry,
        infer::operator_infer::infer_operator,
    };

    #[test]
    fn selection_uses_propagated_keys_after_projection() {
        let catalog = sample_catalog();
        let projection_schema = OutputSchema {
            relation_id: 2,
            columns: vec![BoundColumn {
                slot_id: 10,
                name: "id".to_string(),
                table_alias: None,
                data_type: None,
                nullable: false,
                origin: ColumnOrigin::Derived,
            }],
        };

        let scan_expr = RelExpr::Scan(ScanNode {
            table: "users".to_string(),
            schema: scan_schema(),
        });
        let projection_expr = RelExpr::Projection(ProjectionNode {
            input: Box::new(scan_expr),
            columns: vec![ProjectionColumn {
                expr: BoundScalarExpr::SlotRef(1),
                alias: Some("id".to_string()),
                visibility: Visibility::Visible,
            }],
            schema: projection_schema.clone(),
            is_aggregate: false,
            group_by_count: 0,
        });
        let expr = RelExpr::Selection(SelectionNode {
            input: Box::new(projection_expr),
            condition: BoundScalarExpr::BinaryOp {
                left: Box::new(BoundScalarExpr::SlotRef(10)),
                op: BoundBinaryOp::Eq,
                right: Box::new(BoundScalarExpr::Literal(BoundLiteral::Int {
                    value: 7,
                    raw: "7".to_string(),
                    assignment: false,
                })),
            },
            schema: projection_schema,
        });

        let functions = FunctionRegistry::new(Dialect::Postgres);
        let metadata = infer_operator(&expr, &catalog, Dialect::Postgres, &functions)
            .expect("inference should succeed");

        assert_eq!(
            metadata.cardinality.to_cardinality(),
            Cardinality::AtMostOne
        );
        assert_eq!(metadata.keys.len(), 1);
    }

    fn sample_catalog() -> Catalog {
        Catalog {
            tables: vec![TableSchema {
                name: "users".to_string(),
                original_name: "users".to_string(),
                columns: vec![
                    ColumnSchema {
                        name: "id".to_string(),
                        original_name: "id".to_string(),
                        data_type: sqlex_common::types::DataType::Int,
                        nullable: false,
                    },
                    ColumnSchema {
                        name: "name".to_string(),
                        original_name: "name".to_string(),
                        data_type: sqlex_common::types::DataType::Text,
                        nullable: false,
                    },
                ],
                primary_key: Some(KeyConstraint {
                    name: None,
                    columns: vec!["id".to_string()],
                }),
                unique_keys: Vec::new(),
                foreign_keys: Vec::new(),
            }],
        }
    }

    fn scan_schema() -> OutputSchema {
        OutputSchema {
            relation_id: 1,
            columns: vec![
                BoundColumn {
                    slot_id: 1,
                    name: "id".to_string(),
                    table_alias: Some("users".to_string()),
                    data_type: Some(sqlex_common::types::DataType::Int),
                    nullable: false,
                    origin: ColumnOrigin::Base {
                        table: "users".to_string(),
                        column: "id".to_string(),
                    },
                },
                BoundColumn {
                    slot_id: 2,
                    name: "name".to_string(),
                    table_alias: Some("users".to_string()),
                    data_type: Some(sqlex_common::types::DataType::Text),
                    nullable: false,
                    origin: ColumnOrigin::Base {
                        table: "users".to_string(),
                        column: "name".to_string(),
                    },
                },
            ],
        }
    }
}
