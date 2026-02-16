use std::collections::{HashMap, HashSet};

use sqlex_analyzer::extension::DataTypeExt;
use sqlex_common::types::DataType;

use crate::{
    algebraizer::model::{
        relation::{
            AggregationNode, AliasNode, JoinKind, JoinNode, LimitNode, ProjectionNode, Relation,
            SetOp, SetOpNode, SortNode, WindowNode,
        },
        schema::{ColumnOrigin as BoundColumnOrigin, OutputSchema},
    },
    catalog::Catalog,
    diagnostics::{Diagnostic, Phase},
    infer::{
        Inferencer,
        cardinality::{CardInterval, MaxRows, MinRows},
        metadata::{ColumnOrigin, InferColumn, InferMetadata, ResolvedKey},
    },
};

impl Inferencer<'_> {
    pub(crate) fn infer_operator(&self, relation: &Relation) -> Result<InferMetadata, Diagnostic> {
        self.infer_operator_with_outer_scopes(relation, &[])
    }

    pub(crate) fn infer_operator_with_outer_scopes(
        &self,
        relation: &Relation,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        match relation {
            Relation::Scan(node) => self.infer_scan(node),
            Relation::Values(_) => Ok(InferMetadata {
                columns: Vec::new(),
                cardinality: CardInterval::exactly_one(),
                keys: Vec::new(),
            }),
            Relation::Selection(node) => self.infer_selection(node, outer_scopes),
            Relation::Aggregation(node) => self.infer_aggregation(node, outer_scopes),
            Relation::Window(node) => self.infer_window(node, outer_scopes),
            Relation::Projection(node) => self.infer_projection(node, outer_scopes),
            Relation::Join(node) => self.infer_join(node, outer_scopes),
            Relation::Distinct(node) => self.infer_distinct(node, outer_scopes),
            Relation::Sort(node) => self.infer_sort(node, outer_scopes),
            Relation::Limit(node) => self.infer_limit(node, outer_scopes),
            Relation::Alias(node) => self.infer_alias(node, outer_scopes),
            Relation::SetOperation(node) => self.infer_set_operation(node, outer_scopes),
        }
    }

    fn infer_scan(
        &self,
        node: &crate::algebraizer::model::relation::ScanNode,
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

        let keys = resolve_scan_keys(node, self.catalog);

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

    fn infer_selection(
        &self,
        node: &crate::algebraizer::model::relation::SelectionNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        let mut child = self.infer_operator_with_outer_scopes(&node.input, outer_scopes)?;

        let _ = self.infer_scalar(&node.condition, &child.columns, outer_scopes)?;

        if condition_implies_empty_result(&node.condition, &child.keys, &child.columns) {
            child.cardinality = CardInterval::exactly_zero();
            return Ok(child);
        }

        if let Relation::Join(join_node) = node.input.as_ref() {
            child.cardinality = self.refine_join_cardinality_from_selection(
                child.cardinality,
                join_node,
                &node.condition,
                outer_scopes,
            )?;
        }

        if selection_is_at_most_one(&node.condition, &child.keys, &child.columns) {
            child.cardinality = child.cardinality.constrain_at_most_one();
        }

        Ok(child)
    }

    fn infer_aggregation(
        &self,
        node: &AggregationNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        let child = self.infer_operator_with_outer_scopes(&node.input, outer_scopes)?;
        for projection in &node.group_by {
            let _ = self.infer_scalar(&projection.expr, &child.columns, outer_scopes)?;
        }
        for projection in &node.aggregates {
            let _ = self.infer_scalar(&projection.expr, &child.columns, outer_scopes)?;
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
        &self,
        node: &WindowNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        let child = self.infer_operator_with_outer_scopes(&node.input, outer_scopes)?;
        for projection in &node.window_exprs {
            let _ = self.infer_scalar(&projection.expr, &child.columns, outer_scopes)?;
        }

        Ok(InferMetadata {
            columns: align_columns_to_schema(&child.columns, &node.schema),
            cardinality: child.cardinality,
            keys: child.keys,
        })
    }

    fn infer_projection(
        &self,
        node: &ProjectionNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        let child = self.infer_operator_with_outer_scopes(&node.input, outer_scopes)?;

        let mut columns = Vec::with_capacity(node.columns.len());
        let mut slot_mapping = HashMap::new();
        for (index, projection_column) in node.columns.iter().enumerate() {
            let scalar =
                self.infer_scalar(&projection_column.expr, &child.columns, outer_scopes)?;
            let output_slot_id = node.schema.columns.get(index).map(|column| column.slot_id);
            let output_name = projection_column.alias.clone().ok_or_else(|| {
                Diagnostic::new(
                    "I4201",
                    Phase::Infer,
                    "projection column alias was not assigned during planning",
                )
            })?;
            if let (
                crate::algebraizer::model::expression::Expression::SlotRef(input_slot_id),
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
        &self,
        node: &JoinNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        let left = self.infer_operator_with_outer_scopes(&node.left, outer_scopes)?;
        let right = self.infer_operator_with_outer_scopes(&node.right, outer_scopes)?;

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

        let cardinality = infer_join_cardinality_without_condition(
            node.kind.clone(),
            left.cardinality,
            right.cardinality,
        )?;

        Ok(InferMetadata {
            columns,
            cardinality,
            keys: Vec::new(),
        })
    }

    fn infer_distinct(
        &self,
        node: &crate::algebraizer::model::relation::DistinctNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        let mut child = self.infer_operator_with_outer_scopes(&node.input, outer_scopes)?;
        let output_columns = align_columns_to_schema(&child.columns, &node.schema);
        child.keys = slots_key(output_columns.iter().filter_map(|column| column.slot_id));

        Ok(InferMetadata {
            columns: output_columns,
            cardinality: child.cardinality,
            keys: child.keys,
        })
    }

    fn infer_sort(
        &self,
        node: &SortNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        let child = self.infer_operator_with_outer_scopes(&node.input, outer_scopes)?;
        for key in &node.keys {
            let _ = self.infer_scalar(&key.expr, &child.columns, outer_scopes)?;
        }
        Ok(InferMetadata {
            columns: align_columns_to_schema(&child.columns, &node.schema),
            cardinality: child.cardinality,
            keys: child.keys,
        })
    }

    fn infer_alias(
        &self,
        node: &AliasNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        let child = self.infer_operator_with_outer_scopes(&node.input, outer_scopes)?;
        Ok(InferMetadata {
            columns: align_columns_to_schema(&child.columns, &node.schema),
            cardinality: child.cardinality,
            keys: child.keys,
        })
    }

    fn infer_limit(
        &self,
        node: &LimitNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        let mut child = self.infer_operator_with_outer_scopes(&node.input, outer_scopes)?;

        child.cardinality = infer_limit_cardinality(child.cardinality, node.limit, node.offset)?;

        Ok(child)
    }

    fn infer_set_operation(
        &self,
        node: &SetOpNode,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<InferMetadata, Diagnostic> {
        let left = self.infer_operator_with_outer_scopes(&node.left, outer_scopes)?;
        let right = self.infer_operator_with_outer_scopes(&node.right, outer_scopes)?;
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
                self.dialect,
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
                nullable: set_operation_output_nullable(
                    node.op.clone(),
                    left_column.nullable,
                    right_column.nullable,
                ),
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
}

fn infer_set_operation_cardinality(
    op: SetOp,
    _all: bool,
    left: CardInterval,
    right: CardInterval,
) -> Result<CardInterval, Diagnostic> {
    match op {
        SetOp::Union => CardInterval::try_new(
            lower_or(left.min(), right.min()),
            upper_add(left.max(), right.max()),
            "infer_set_operation_cardinality::union",
        ),
        SetOp::Intersect => CardInterval::try_new(
            MinRows::Zero,
            upper_min(left.max(), right.max()),
            "infer_set_operation_cardinality::intersect",
        ),
        SetOp::Except => CardInterval::try_new(
            if matches!(right.max(), MaxRows::Zero) {
                left.min()
            } else {
                MinRows::Zero
            },
            left.max(),
            "infer_set_operation_cardinality::except",
        ),
    }
}

fn resolve_scan_keys(
    node: &crate::algebraizer::model::relation::ScanNode,
    catalog: &Catalog,
) -> Vec<ResolvedKey> {
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

fn infer_join_cardinality_without_condition(
    kind: JoinKind,
    left: CardInterval,
    right: CardInterval,
) -> Result<CardInterval, Diagnostic> {
    match kind {
        JoinKind::Inner => CardInterval::try_new(
            MinRows::Zero,
            upper_mul(left.max(), right.max()),
            "infer_join_cardinality_without_condition::inner",
        ),
        JoinKind::Left => {
            let max = if matches!(right.max(), MaxRows::Zero) {
                left.max()
            } else {
                upper_mul(left.max(), right.max())
            };
            CardInterval::try_new(
                left.min(),
                max,
                "infer_join_cardinality_without_condition::left",
            )
        },
        JoinKind::Right => {
            let max = if matches!(left.max(), MaxRows::Zero) {
                right.max()
            } else {
                upper_mul(left.max(), right.max())
            };
            CardInterval::try_new(
                right.min(),
                max,
                "infer_join_cardinality_without_condition::right",
            )
        },
        JoinKind::Full => {
            let max = if matches!(left.max(), MaxRows::Zero) {
                right.max()
            } else if matches!(right.max(), MaxRows::Zero) {
                left.max()
            } else {
                MaxRows::Many
            };
            CardInterval::try_new(
                lower_or(left.min(), right.min()),
                max,
                "infer_join_cardinality_without_condition::full",
            )
        },
        JoinKind::Cross => CardInterval::try_new(
            lower_and(left.min(), right.min()),
            upper_mul(left.max(), right.max()),
            "infer_join_cardinality_without_condition::cross",
        ),
    }
}

fn infer_limit_cardinality(
    input: CardInterval,
    limit: Option<u64>,
    offset: Option<u64>,
) -> Result<CardInterval, Diagnostic> {
    let has_offset = offset.is_some_and(|value| value > 0);
    match limit {
        Some(0) => Ok(CardInterval::exactly_zero()),
        Some(1) if has_offset => {
            if has_at_most_one_row(input) {
                Ok(CardInterval::exactly_zero())
            } else {
                Ok(CardInterval::at_most_one())
            }
        },
        Some(1) => Ok(input.constrain_at_most_one()),
        _ if has_offset => {
            if has_at_most_one_row(input) {
                Ok(CardInterval::exactly_zero())
            } else {
                Ok(input.drop_lower_bound())
            }
        },
        _ => Ok(input),
    }
}

fn has_at_most_one_row(interval: CardInterval) -> bool {
    matches!(interval.max(), MaxRows::Zero | MaxRows::One)
}

fn set_operation_output_nullable(op: SetOp, left: bool, right: bool) -> bool {
    match op {
        SetOp::Union | SetOp::Intersect => left || right,
        SetOp::Except => left,
    }
}

fn condition_implies_empty_result(
    condition: &crate::algebraizer::model::expression::Expression,
    input_keys: &[ResolvedKey],
    input_columns: &[InferColumn],
) -> bool {
    always_false_condition(condition)
        || has_contradictory_equalities(condition)
        || has_full_key_is_null_on_proven_non_nullable_key(condition, input_keys, input_columns)
}

fn has_contradictory_equalities(
    condition: &crate::algebraizer::model::expression::Expression,
) -> bool {
    let mut equalities: HashMap<u32, crate::algebraizer::model::expression::BoundLiteral> =
        HashMap::new();
    collect_contradictory_equalities(condition, &mut equalities)
}

fn collect_contradictory_equalities(
    expr: &crate::algebraizer::model::expression::Expression,
    equalities: &mut HashMap<u32, crate::algebraizer::model::expression::BoundLiteral>,
) -> bool {
    match expr {
        crate::algebraizer::model::expression::Expression::BinaryOp { left, op, right } => match op
        {
            crate::algebraizer::model::expression::BoundBinaryOp::And => {
                collect_contradictory_equalities(left, equalities)
                    || collect_contradictory_equalities(right, equalities)
            },
            crate::algebraizer::model::expression::BoundBinaryOp::Eq => {
                equality_constraint_conflicts(left, right, equalities)
                    || equality_constraint_conflicts(right, left, equalities)
            },
            _ => false,
        },
        _ => false,
    }
}

fn equality_constraint_conflicts(
    left: &crate::algebraizer::model::expression::Expression,
    right: &crate::algebraizer::model::expression::Expression,
    equalities: &mut HashMap<u32, crate::algebraizer::model::expression::BoundLiteral>,
) -> bool {
    let crate::algebraizer::model::expression::Expression::SlotRef(slot_id) = left else {
        return false;
    };
    let Some(literal) = extract_comparable_literal(right) else {
        return false;
    };

    if let Some(existing) = equalities.get(slot_id) {
        return matches!(literal_equal(existing, literal), Some(false));
    }

    equalities.insert(*slot_id, literal.clone());
    false
}

fn extract_comparable_literal(
    expr: &crate::algebraizer::model::expression::Expression,
) -> Option<&crate::algebraizer::model::expression::BoundLiteral> {
    match expr {
        crate::algebraizer::model::expression::Expression::Literal(literal) => Some(literal),
        crate::algebraizer::model::expression::Expression::Cast { expr, .. } => {
            extract_comparable_literal(expr)
        },
        _ => None,
    }
}

fn has_full_key_is_null_on_proven_non_nullable_key(
    condition: &crate::algebraizer::model::expression::Expression,
    input_keys: &[ResolvedKey],
    input_columns: &[InferColumn],
) -> bool {
    let mut constraints = HashMap::<u32, SingleValueConstraint>::new();
    if !collect_single_value_constraints(condition, &mut constraints) {
        return false;
    }

    input_keys.iter().any(|key| {
        !key.slot_ids.is_empty()
            && key.slot_ids.iter().all(|slot_id| {
                matches!(
                    constraints.get(slot_id),
                    Some(SingleValueConstraint::IsNull)
                ) && column_is_non_nullable(input_columns, *slot_id)
            })
    })
}

fn column_is_non_nullable(columns: &[InferColumn], slot_id: u32) -> bool {
    columns
        .iter()
        .find(|column| column.slot_id.is_some_and(|value| value == slot_id))
        .is_some_and(|column| !column.nullable)
}

impl Inferencer<'_> {
    fn refine_join_cardinality_from_selection(
        &self,
        current: CardInterval,
        join_node: &JoinNode,
        condition: &crate::algebraizer::model::expression::Expression,
        outer_scopes: &[Vec<InferColumn>],
    ) -> Result<CardInterval, Diagnostic> {
        let left = self.infer_operator_with_outer_scopes(&join_node.left, outer_scopes)?;
        let right = self.infer_operator_with_outer_scopes(&join_node.right, outer_scopes)?;

        let Some(join_pairs) =
            extract_join_equijoin_pairs(condition, &left.columns, &right.columns)
        else {
            return Ok(current);
        };

        let at_most_one_right_per_left =
            join_pairs_cover_non_nullable_key(&join_pairs, &right.keys, &right.columns, false);
        let at_most_one_left_per_right =
            join_pairs_cover_non_nullable_key(&join_pairs, &left.keys, &left.columns, true);

        let mut max = current.max();
        match join_node.kind {
            JoinKind::Inner => {
                if at_most_one_right_per_left {
                    max = upper_min(max, left.cardinality.max());
                }
                if at_most_one_left_per_right {
                    max = upper_min(max, right.cardinality.max());
                }
            },
            JoinKind::Left => {
                if at_most_one_right_per_left {
                    max = upper_min(max, left.cardinality.max());
                }
            },
            JoinKind::Right => {
                if at_most_one_left_per_right {
                    max = upper_min(max, right.cardinality.max());
                }
            },
            JoinKind::Full | JoinKind::Cross => {},
        }

        CardInterval::try_new(current.min(), max, "refine_join_cardinality_from_selection")
    }
}

fn extract_join_equijoin_pairs(
    condition: &crate::algebraizer::model::expression::Expression,
    left_columns: &[InferColumn],
    right_columns: &[InferColumn],
) -> Option<Vec<(u32, u32)>> {
    let left_slots: HashSet<u32> = left_columns
        .iter()
        .filter_map(|column| column.slot_id)
        .collect();
    let right_slots: HashSet<u32> = right_columns
        .iter()
        .filter_map(|column| column.slot_id)
        .collect();
    if left_slots.is_empty() || right_slots.is_empty() {
        return None;
    }

    let mut pairs = HashSet::new();
    if !collect_join_equijoin_pairs(condition, &left_slots, &right_slots, &mut pairs) {
        return None;
    }

    if pairs.is_empty() {
        None
    } else {
        Some(pairs.into_iter().collect())
    }
}

fn collect_join_equijoin_pairs(
    expr: &crate::algebraizer::model::expression::Expression,
    left_slots: &HashSet<u32>,
    right_slots: &HashSet<u32>,
    pairs: &mut HashSet<(u32, u32)>,
) -> bool {
    match expr {
        crate::algebraizer::model::expression::Expression::BinaryOp { left, op, right } => match op
        {
            crate::algebraizer::model::expression::BoundBinaryOp::And => {
                collect_join_equijoin_pairs(left, left_slots, right_slots, pairs)
                    && collect_join_equijoin_pairs(right, left_slots, right_slots, pairs)
            },
            crate::algebraizer::model::expression::BoundBinaryOp::Eq => {
                let (
                    crate::algebraizer::model::expression::Expression::SlotRef(left_slot),
                    crate::algebraizer::model::expression::Expression::SlotRef(right_slot),
                ) = (left.as_ref(), right.as_ref())
                else {
                    return false;
                };

                if left_slots.contains(left_slot) && right_slots.contains(right_slot) {
                    pairs.insert((*left_slot, *right_slot));
                    true
                } else if left_slots.contains(right_slot) && right_slots.contains(left_slot) {
                    pairs.insert((*right_slot, *left_slot));
                    true
                } else {
                    false
                }
            },
            _ => false,
        },
        _ => false,
    }
}

fn join_pairs_cover_non_nullable_key(
    join_pairs: &[(u32, u32)],
    keys: &[ResolvedKey],
    columns: &[InferColumn],
    use_left_slot: bool,
) -> bool {
    let constrained_slots: HashSet<u32> = if use_left_slot {
        join_pairs.iter().map(|(left_slot, _)| *left_slot).collect()
    } else {
        join_pairs
            .iter()
            .map(|(_, right_slot)| *right_slot)
            .collect()
    };

    keys.iter().any(|key| {
        !key.slot_ids.is_empty()
            && key
                .slot_ids
                .iter()
                .all(|slot_id| constrained_slots.contains(slot_id))
            && key
                .slot_ids
                .iter()
                .all(|slot_id| column_is_non_nullable(columns, *slot_id))
    })
}

fn upper_min(left: MaxRows, right: MaxRows) -> MaxRows {
    match (left, right) {
        (MaxRows::Zero, _) | (_, MaxRows::Zero) => MaxRows::Zero,
        (MaxRows::One, _) | (_, MaxRows::One) => MaxRows::One,
        (MaxRows::Many, MaxRows::Many) => MaxRows::Many,
    }
}

fn upper_add(left: MaxRows, right: MaxRows) -> MaxRows {
    match (left, right) {
        (MaxRows::Zero, MaxRows::Zero) => MaxRows::Zero,
        (MaxRows::Zero, MaxRows::One) | (MaxRows::One, MaxRows::Zero) => MaxRows::One,
        (MaxRows::One, MaxRows::One) => MaxRows::Many,
        _ => MaxRows::Many,
    }
}

fn upper_mul(left: MaxRows, right: MaxRows) -> MaxRows {
    match (left, right) {
        (MaxRows::Zero, _) | (_, MaxRows::Zero) => MaxRows::Zero,
        (MaxRows::One, MaxRows::One) => MaxRows::One,
        _ => MaxRows::Many,
    }
}

fn lower_or(left: MinRows, right: MinRows) -> MinRows {
    if matches!(left, MinRows::One) || matches!(right, MinRows::One) {
        MinRows::One
    } else {
        MinRows::Zero
    }
}

fn lower_and(left: MinRows, right: MinRows) -> MinRows {
    if matches!(left, MinRows::One) && matches!(right, MinRows::One) {
        MinRows::One
    } else {
        MinRows::Zero
    }
}

fn always_false_condition(condition: &crate::algebraizer::model::expression::Expression) -> bool {
    match condition {
        crate::algebraizer::model::expression::Expression::Literal(
            crate::algebraizer::model::expression::BoundLiteral::Bool(value),
        ) => !*value,
        crate::algebraizer::model::expression::Expression::BinaryOp { left, op, right } => match op
        {
            crate::algebraizer::model::expression::BoundBinaryOp::Eq => {
                literal_comparison_false(left, right, true)
            },
            crate::algebraizer::model::expression::BoundBinaryOp::NotEq => {
                literal_comparison_false(left, right, false)
            },
            _ => false,
        },
        _ => false,
    }
}

fn literal_comparison_false(
    left: &crate::algebraizer::model::expression::Expression,
    right: &crate::algebraizer::model::expression::Expression,
    is_eq: bool,
) -> bool {
    let crate::algebraizer::model::expression::Expression::Literal(left_literal) = left else {
        return false;
    };
    let crate::algebraizer::model::expression::Expression::Literal(right_literal) = right else {
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
    left: &crate::algebraizer::model::expression::BoundLiteral,
    right: &crate::algebraizer::model::expression::BoundLiteral,
) -> Option<bool> {
    match (left, right) {
        (
            crate::algebraizer::model::expression::BoundLiteral::Null,
            crate::algebraizer::model::expression::BoundLiteral::Null,
        ) => Some(true),
        (
            crate::algebraizer::model::expression::BoundLiteral::Bool(left_value),
            crate::algebraizer::model::expression::BoundLiteral::Bool(right_value),
        ) => Some(left_value == right_value),
        (
            crate::algebraizer::model::expression::BoundLiteral::Int {
                value: left_value, ..
            },
            crate::algebraizer::model::expression::BoundLiteral::Int {
                value: right_value, ..
            },
        ) => Some(left_value == right_value),
        (
            crate::algebraizer::model::expression::BoundLiteral::Float(left_value),
            crate::algebraizer::model::expression::BoundLiteral::Float(right_value),
        ) => Some(left_value.to_bits() == right_value.to_bits()),
        (
            crate::algebraizer::model::expression::BoundLiteral::String(left_value),
            crate::algebraizer::model::expression::BoundLiteral::String(right_value),
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
    condition: &crate::algebraizer::model::expression::Expression,
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
    expr: &crate::algebraizer::model::expression::Expression,
    constraints: &mut HashMap<u32, SingleValueConstraint>,
) -> bool {
    match expr {
        crate::algebraizer::model::expression::Expression::BinaryOp { left, op, right } => match op
        {
            crate::algebraizer::model::expression::BoundBinaryOp::And => {
                collect_single_value_constraints(left, constraints)
                    && collect_single_value_constraints(right, constraints)
            },
            crate::algebraizer::model::expression::BoundBinaryOp::Or => false,
            crate::algebraizer::model::expression::BoundBinaryOp::Eq => {
                if let Some(slot_id) = slot_id_equals_single_value(left, right) {
                    set_constraint(constraints, slot_id, SingleValueConstraint::EqLike);
                } else if let Some(slot_id) = slot_id_equals_single_value(right, left) {
                    set_constraint(constraints, slot_id, SingleValueConstraint::EqLike);
                }
                true
            },
            _ => true,
        },
        crate::algebraizer::model::expression::Expression::InList {
            expr,
            list,
            negated,
        } => {
            if !*negated && list.len() == 1 {
                if let crate::algebraizer::model::expression::Expression::SlotRef(slot_id) =
                    expr.as_ref()
                {
                    set_constraint(constraints, *slot_id, SingleValueConstraint::EqLike);
                }
            }
            true
        },
        crate::algebraizer::model::expression::Expression::IsNull { expr, negated } => {
            if !*negated {
                if let crate::algebraizer::model::expression::Expression::SlotRef(slot_id) =
                    expr.as_ref()
                {
                    set_constraint(constraints, *slot_id, SingleValueConstraint::IsNull);
                }
            }
            true
        },
        _ => true,
    }
}

fn slot_id_equals_single_value(
    left: &crate::algebraizer::model::expression::Expression,
    right: &crate::algebraizer::model::expression::Expression,
) -> Option<u32> {
    let crate::algebraizer::model::expression::Expression::SlotRef(slot_id) = left else {
        return None;
    };
    if is_single_value_expr(right) {
        Some(*slot_id)
    } else {
        None
    }
}

fn is_single_value_expr(expr: &crate::algebraizer::model::expression::Expression) -> bool {
    match expr {
        crate::algebraizer::model::expression::Expression::Literal(_) => true,
        crate::algebraizer::model::expression::Expression::Cast { expr, .. } => {
            is_single_value_expr(expr)
        },
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
        algebraizer::model::{
            expression::{BoundBinaryOp, BoundLiteral, Expression},
            relation::{
                JoinKind, JoinNode, LimitNode, ProjectionNode, Relation, ScanNode, SelectionNode,
                SetOp,
            },
            schema::{BoundColumn, ColumnOrigin, OutputSchema, ProjectionColumn, Visibility},
        },
        catalog::{
            Catalog,
            model::{ColumnSchema, KeyConstraint, TableSchema},
        },
        functions::FunctionRegistry,
        infer::{
            Inferencer,
            cardinality::CardInterval,
            operator_infer::{infer_limit_cardinality, infer_set_operation_cardinality},
        },
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

        let scan_relation = Relation::Scan(ScanNode {
            table: "users".to_string(),
            schema: scan_schema(),
        });
        let projection_relation = Relation::Projection(ProjectionNode {
            input: Box::new(scan_relation),
            columns: vec![ProjectionColumn {
                expr: Expression::SlotRef(1),
                alias: Some("id".to_string()),
                visibility: Visibility::Visible,
            }],
            schema: projection_schema.clone(),
        });
        let relation = Relation::Selection(SelectionNode {
            input: Box::new(projection_relation),
            condition: Expression::BinaryOp {
                left: Box::new(Expression::SlotRef(10)),
                op: BoundBinaryOp::Eq,
                right: Box::new(Expression::Literal(BoundLiteral::Int {
                    value: 7,
                    raw: "7".to_string(),
                    assignment: false,
                })),
            },
            schema: projection_schema,
        });

        let functions = FunctionRegistry::new(Dialect::Postgres);
        let inferencer = Inferencer::new(Dialect::Postgres, &catalog, &functions);
        let metadata = inferencer
            .infer_operator(&relation)
            .expect("inference should succeed");

        assert_eq!(
            metadata.cardinality.to_cardinality(),
            Cardinality::AtMostOne
        );
        assert_eq!(metadata.keys.len(), 1);
    }

    #[test]
    fn selection_false_condition_is_exactly_zero() {
        let catalog = sample_catalog();
        let schema = scan_schema();
        let relation = Relation::Selection(SelectionNode {
            input: Box::new(Relation::Scan(ScanNode {
                table: "users".to_string(),
                schema: schema.clone(),
            })),
            condition: Expression::Literal(BoundLiteral::Bool(false)),
            schema,
        });

        let functions = FunctionRegistry::new(Dialect::Postgres);
        let inferencer = Inferencer::new(Dialect::Postgres, &catalog, &functions);
        let metadata = inferencer
            .infer_operator(&relation)
            .expect("inference should succeed");

        assert_eq!(
            metadata.cardinality.to_cardinality(),
            Cardinality::ExactlyZero
        );
    }

    #[test]
    fn selection_contradictory_equalities_is_exactly_zero() {
        let catalog = sample_catalog();
        let schema = scan_schema();
        let relation = Relation::Selection(SelectionNode {
            input: Box::new(Relation::Scan(ScanNode {
                table: "users".to_string(),
                schema: schema.clone(),
            })),
            condition: Expression::BinaryOp {
                left: Box::new(Expression::BinaryOp {
                    left: Box::new(Expression::SlotRef(1)),
                    op: BoundBinaryOp::Eq,
                    right: Box::new(Expression::Literal(BoundLiteral::Int {
                        value: 1,
                        raw: "1".to_string(),
                        assignment: false,
                    })),
                }),
                op: BoundBinaryOp::And,
                right: Box::new(Expression::BinaryOp {
                    left: Box::new(Expression::SlotRef(1)),
                    op: BoundBinaryOp::Eq,
                    right: Box::new(Expression::Literal(BoundLiteral::Int {
                        value: 2,
                        raw: "2".to_string(),
                        assignment: false,
                    })),
                }),
            },
            schema,
        });

        let functions = FunctionRegistry::new(Dialect::Postgres);
        let inferencer = Inferencer::new(Dialect::Postgres, &catalog, &functions);
        let metadata = inferencer
            .infer_operator(&relation)
            .expect("inference should succeed");

        assert_eq!(
            metadata.cardinality.to_cardinality(),
            Cardinality::ExactlyZero
        );
    }

    #[test]
    fn selection_non_nullable_key_is_null_is_exactly_zero() {
        let catalog = sample_catalog();
        let schema = scan_schema();
        let relation = Relation::Selection(SelectionNode {
            input: Box::new(Relation::Scan(ScanNode {
                table: "users".to_string(),
                schema: schema.clone(),
            })),
            condition: Expression::IsNull {
                expr: Box::new(Expression::SlotRef(1)),
                negated: false,
            },
            schema,
        });

        let functions = FunctionRegistry::new(Dialect::Postgres);
        let inferencer = Inferencer::new(Dialect::Postgres, &catalog, &functions);
        let metadata = inferencer
            .infer_operator(&relation)
            .expect("inference should succeed");

        assert_eq!(
            metadata.cardinality.to_cardinality(),
            Cardinality::ExactlyZero
        );
    }

    #[test]
    fn limit_cardinality_rules_follow_design() {
        assert_eq!(
            infer_limit_cardinality(CardInterval::zero_or_more(), Some(0), None)
                .expect("limit inference should succeed")
                .to_cardinality(),
            Cardinality::ExactlyZero
        );
        assert_eq!(
            infer_limit_cardinality(CardInterval::at_most_one(), None, Some(1))
                .expect("limit inference should succeed")
                .to_cardinality(),
            Cardinality::ExactlyZero
        );
        assert_eq!(
            infer_limit_cardinality(CardInterval::one_or_more(), Some(1), Some(1))
                .expect("limit inference should succeed")
                .to_cardinality(),
            Cardinality::AtMostOne
        );
    }

    #[test]
    fn set_operation_cardinality_follows_design() {
        assert_eq!(
            infer_set_operation_cardinality(
                SetOp::Union,
                false,
                CardInterval::exactly_one(),
                CardInterval::exactly_zero(),
            )
            .expect("set-op inference should succeed")
            .to_cardinality(),
            Cardinality::ExactlyOne
        );
        assert_eq!(
            infer_set_operation_cardinality(
                SetOp::Intersect,
                false,
                CardInterval::exactly_one(),
                CardInterval::exactly_one(),
            )
            .expect("set-op inference should succeed")
            .to_cardinality(),
            Cardinality::AtMostOne
        );
        assert_eq!(
            infer_set_operation_cardinality(
                SetOp::Except,
                false,
                CardInterval::exactly_one(),
                CardInterval::exactly_one(),
            )
            .expect("set-op inference should succeed")
            .to_cardinality(),
            Cardinality::AtMostOne
        );
    }

    #[test]
    fn selection_over_join_uses_companion_refinement() {
        let catalog = sample_catalog();
        let users_scan = Relation::Scan(ScanNode {
            table: "users".to_string(),
            schema: scan_schema(),
        });
        let limited_users = Relation::Limit(LimitNode {
            input: Box::new(users_scan),
            limit: Some(1),
            offset: None,
            schema: scan_schema(),
        });
        let orders_scan = Relation::Scan(ScanNode {
            table: "orders".to_string(),
            schema: orders_scan_schema(),
        });

        let join_schema = OutputSchema {
            relation_id: 3,
            columns: {
                let mut columns = scan_schema().columns;
                columns.extend(orders_scan_schema().columns);
                columns
            },
        };
        let join_relation = Relation::Join(JoinNode {
            left: Box::new(limited_users),
            right: Box::new(orders_scan),
            kind: JoinKind::Inner,
            schema: join_schema.clone(),
        });
        let relation = Relation::Selection(SelectionNode {
            input: Box::new(join_relation),
            condition: Expression::BinaryOp {
                left: Box::new(Expression::SlotRef(1)),
                op: BoundBinaryOp::Eq,
                right: Box::new(Expression::SlotRef(3)),
            },
            schema: join_schema,
        });

        let functions = FunctionRegistry::new(Dialect::Postgres);
        let inferencer = Inferencer::new(Dialect::Postgres, &catalog, &functions);
        let metadata = inferencer
            .infer_operator(&relation)
            .expect("inference should succeed");

        assert_eq!(
            metadata.cardinality.to_cardinality(),
            Cardinality::AtMostOne
        );
    }

    fn sample_catalog() -> Catalog {
        Catalog {
            tables: vec![
                TableSchema {
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
                },
                TableSchema {
                    name: "orders".to_string(),
                    original_name: "orders".to_string(),
                    columns: vec![
                        ColumnSchema {
                            name: "id".to_string(),
                            original_name: "id".to_string(),
                            data_type: sqlex_common::types::DataType::Int,
                            nullable: false,
                        },
                        ColumnSchema {
                            name: "user_id".to_string(),
                            original_name: "user_id".to_string(),
                            data_type: sqlex_common::types::DataType::Int,
                            nullable: false,
                        },
                    ],
                    primary_key: Some(KeyConstraint {
                        name: None,
                        columns: vec!["id".to_string()],
                    }),
                    unique_keys: Vec::new(),
                    foreign_keys: Vec::new(),
                },
            ],
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

    fn orders_scan_schema() -> OutputSchema {
        OutputSchema {
            relation_id: 2,
            columns: vec![
                BoundColumn {
                    slot_id: 3,
                    name: "id".to_string(),
                    table_alias: Some("orders".to_string()),
                    data_type: Some(sqlex_common::types::DataType::Int),
                    nullable: false,
                    origin: ColumnOrigin::Base {
                        table: "orders".to_string(),
                        column: "id".to_string(),
                    },
                },
                BoundColumn {
                    slot_id: 4,
                    name: "user_id".to_string(),
                    table_alias: Some("orders".to_string()),
                    data_type: Some(sqlex_common::types::DataType::Int),
                    nullable: false,
                    origin: ColumnOrigin::Base {
                        table: "orders".to_string(),
                        column: "user_id".to_string(),
                    },
                },
            ],
        }
    }
}
