use std::collections::HashMap;

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
        metadata::{ColumnOrigin, InferColumn, InferMetadata},
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
        RelExpr::Scan(node) => infer_scan(node),
        RelExpr::Values(_) => Ok(InferMetadata {
            columns: Vec::new(),
            cardinality: CardInterval {
                min: MinRows::One,
                max: MaxRows::One,
            },
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
                child.cardinality = CardInterval {
                    min: MinRows::Zero,
                    max: MaxRows::One,
                };
            }
            if selection_is_at_most_one(&node.condition, &child.columns, catalog) {
                child.cardinality.max = MaxRows::One;
                child.cardinality.min = MinRows::Zero;
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
        RelExpr::Distinct(node) => {
            infer_operator_with_outer_scopes(&node.input, catalog, dialect, functions, outer_scopes)
        },
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

fn infer_scan(node: &crate::algebra::expr::ScanNode) -> Result<InferMetadata, Diagnostic> {
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

    Ok(InferMetadata {
        columns,
        cardinality: if node.table.starts_with("__recursive_cte__") {
            CardInterval {
                min: MinRows::One,
                max: MaxRows::Many,
            }
        } else {
            CardInterval {
                min: MinRows::Zero,
                max: MaxRows::Many,
            }
        },
        keys: Vec::new(),
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
        CardInterval {
            min: MinRows::One,
            max: MaxRows::One,
        }
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
        keys: Vec::new(),
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
        keys: Vec::new(),
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
        JoinKind::Left | JoinKind::Right | JoinKind::Full | JoinKind::Inner => CardInterval {
            min: MinRows::Zero,
            max: MaxRows::Many,
        },
        JoinKind::Cross => CardInterval {
            min: MinRows::Zero,
            max: MaxRows::Many,
        },
    };

    Ok(InferMetadata {
        columns,
        cardinality,
        keys: Vec::new(),
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
        keys: Vec::new(),
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
        keys: Vec::new(),
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
        child.cardinality.min = MinRows::Zero;
    }
    if node.limit.is_some_and(|value| value <= 1) {
        child.cardinality = CardInterval {
            min: MinRows::Zero,
            max: MaxRows::One,
        };
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
    );

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
) -> CardInterval {
    match op {
        SetOp::Union => {
            let min = if matches!(left.min, MinRows::One) || matches!(right.min, MinRows::One) {
                MinRows::One
            } else {
                MinRows::Zero
            };
            let max = match (left.max, right.max) {
                (MaxRows::Zero, MaxRows::Zero) => MaxRows::Zero,
                _ => MaxRows::Many,
            };
            if all {
                CardInterval { min, max }
            } else {
                CardInterval { min, max }
            }
        },
        SetOp::Intersect => CardInterval {
            min: if matches!(left.min, MinRows::One) && matches!(right.min, MinRows::One) {
                MinRows::One
            } else {
                MinRows::Zero
            },
            max: min_max_rows(left.max, right.max),
        },
        SetOp::Except => left,
    }
}

fn min_max_rows(left: MaxRows, right: MaxRows) -> MaxRows {
    match (left, right) {
        (MaxRows::Zero, _) | (_, MaxRows::Zero) => MaxRows::Zero,
        (MaxRows::One, MaxRows::One) => MaxRows::One,
        _ => MaxRows::Many,
    }
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
    input_columns: &[InferColumn],
    catalog: &Catalog,
) -> bool {
    let mut constraints = HashMap::<u32, SingleValueConstraint>::new();
    if !collect_single_value_constraints(condition, &mut constraints) {
        return false;
    }

    let mut table_constraints: HashMap<String, HashMap<String, SingleValueConstraint>> =
        HashMap::new();
    for (slot_id, constraint) in constraints {
        let Some(column) = input_columns
            .iter()
            .find(|column| column.slot_id.is_some_and(|value| value == slot_id))
        else {
            continue;
        };
        let ColumnOrigin::Base {
            table,
            column: column_name,
        } = &column.origin
        else {
            continue;
        };
        table_constraints
            .entry(table.clone())
            .or_default()
            .insert(column_name.clone(), constraint);
    }

    for (table_name, constrained_columns) in table_constraints {
        let Some(table) = catalog.table(&table_name) else {
            continue;
        };
        if key_satisfied(
            table,
            table.primary_key.as_ref().map(|key| key.columns.as_slice()),
            &constrained_columns,
        ) {
            return true;
        }
        for key in &table.unique_keys {
            if key_satisfied(table, Some(key.columns.as_slice()), &constrained_columns) {
                return true;
            }
        }
    }

    false
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
    table: &crate::catalog::model::TableSchema,
    key_columns: Option<&[String]>,
    constrained_columns: &HashMap<String, SingleValueConstraint>,
) -> bool {
    let Some(key_columns) = key_columns else {
        return false;
    };
    if key_columns.is_empty() {
        return false;
    }

    for column_name in key_columns {
        let Some(constraint) = constrained_columns.get(column_name) else {
            return false;
        };
        if *constraint == SingleValueConstraint::EqLike {
            continue;
        }
        let Some(column) = table
            .columns
            .iter()
            .find(|column| &column.name == column_name)
        else {
            return false;
        };
        if column.nullable {
            return false;
        }
    }

    true
}
