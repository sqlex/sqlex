use sqlex_common::dialect::Dialect;

use crate::{
    algebraizer::model::relation::Relation, catalog::Catalog, diagnostics::Diagnostic,
    functions::FunctionRegistry, infer::model::metadata::InferMetadata,
};

mod expression;
pub(crate) mod model;
mod relation;

#[derive(Debug, Clone)]
pub(crate) struct Inferencer<'a> {
    dialect: Dialect,
    catalog: &'a Catalog,
    functions: &'a FunctionRegistry,
}

impl<'a> Inferencer<'a> {
    pub(crate) fn new(
        dialect: Dialect,
        catalog: &'a Catalog,
        functions: &'a FunctionRegistry,
    ) -> Self {
        Self {
            dialect,
            catalog,
            functions,
        }
    }

    pub(crate) fn infer(&self, relation: &Relation) -> Result<InferMetadata, Diagnostic> {
        self.infer_relation(relation)
    }
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
            model::cardinality::CardInterval,
            relation::cardinality::{infer_limit_cardinality, infer_set_operation_cardinality},
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
            .infer_relation(&relation)
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
            .infer_relation(&relation)
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
            .infer_relation(&relation)
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
            .infer_relation(&relation)
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
            .infer_relation(&relation)
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
