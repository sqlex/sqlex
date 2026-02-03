use sqlex_analyzer::AnalyzerError;
use sqlparser::ast::{JoinConstraint, JoinOperator};

use crate::{
    planner::{
        expr::TypedExpr,
        plan::{LogicalNode, PlanNode, PlanNodeColumn},
        scope::Scope,
    },
    schema::Schema,
};

type Result<T> = std::result::Result<T, AnalyzerError>;

/// Join type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinKind {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

/// Join condition
#[derive(Debug, Clone)]
pub enum JoinCondition {
    /// ON expr
    On(Box<TypedExpr>),
    /// USING (col1, col2)
    Using(Vec<String>),
    /// NATURAL JOIN
    Natural,
}

#[derive(Debug, Clone)]
pub struct JoinNode {
    pub kind: JoinKind,
    pub left: Box<dyn PlanNode>,
    pub right: Box<dyn PlanNode>,
    pub condition: Option<JoinCondition>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl JoinNode {
    /// Build from AST JoinOperator
    pub fn from_ast(
        schema: &Schema,
        left: Box<dyn PlanNode>,
        right: Box<dyn PlanNode>,
        join_operator: &JoinOperator,
        left_scope: &Scope,
        right_scope: &Scope,
    ) -> Result<Self> {
        // Convert JoinOperator to JoinKind and extract constraint
        let (kind, constraint) = match join_operator {
            JoinOperator::Inner(constraint) => (JoinKind::Inner, Some(constraint)),
            JoinOperator::LeftOuter(constraint) => (JoinKind::Left, Some(constraint)),
            JoinOperator::RightOuter(constraint) => (JoinKind::Right, Some(constraint)),
            JoinOperator::FullOuter(constraint) => (JoinKind::Full, Some(constraint)),
            JoinOperator::CrossJoin => (JoinKind::Cross, None),
            _ => {
                return Err(AnalyzerError::AnalysisError(
                    "Unsupported join type".to_string(),
                ));
            },
        };

        // Convert JoinConstraint to JoinCondition
        let condition = if let Some(constraint) = constraint {
            Some(Self::build_join_condition(
                constraint,
                left_scope,
                right_scope,
            )?)
        } else {
            None
        };

        Ok(Self::build(schema, left, right, kind, condition))
    }

    /// Build join condition from AST constraint
    fn build_join_condition(
        constraint: &JoinConstraint,
        left_scope: &Scope,
        right_scope: &Scope,
    ) -> Result<JoinCondition> {
        let mut combined_scope = left_scope.clone();
        combined_scope.merge(right_scope.clone());

        match constraint {
            JoinConstraint::On(expr) => {
                let typed = TypedExpr::from_expr(expr, &combined_scope)?;
                Ok(JoinCondition::On(Box::new(typed)))
            },
            JoinConstraint::Using(idents) => Ok(JoinCondition::Using(
                idents
                    .iter()
                    .map(|id| {
                        id.0.iter()
                            .map(|i| i.value.clone())
                            .collect::<Vec<_>>()
                            .join(".")
                    })
                    .collect(),
            )),
            JoinConstraint::Natural => Ok(JoinCondition::Natural),
            JoinConstraint::None => {
                panic!("Constraints None shouldn't happen for Inner/Outer join")
            },
        }
    }

    pub fn build(
        schema: &Schema,
        left: Box<dyn PlanNode>,
        right: Box<dyn PlanNode>,
        kind: JoinKind,
        condition: Option<JoinCondition>,
    ) -> Self {
        let mut left_cols = left.columns().to_vec();
        let mut right_cols = right.columns().to_vec();

        // Determine nullability for JOIN columns
        let (force_left_nullable, force_right_nullable) = match kind {
            JoinKind::Inner | JoinKind::Cross => (false, false),
            JoinKind::Left => {
                if check_fk_guarantee(
                    JoinKind::Left,
                    left.as_ref(),
                    right.as_ref(),
                    &condition,
                    schema,
                ) {
                    (false, false)
                } else {
                    (false, true)
                }
            },
            JoinKind::Right => {
                if check_fk_guarantee(
                    JoinKind::Right,
                    left.as_ref(),
                    right.as_ref(),
                    &condition,
                    schema,
                ) {
                    (false, false)
                } else {
                    (true, false)
                }
            },
            JoinKind::Full => (true, true),
        };

        if force_left_nullable {
            for col in &mut left_cols {
                col.nullability = true;
            }
        }

        if force_right_nullable {
            for col in &mut right_cols {
                col.nullability = true;
            }
        }

        let mut output_columns = left_cols;
        output_columns.extend(right_cols);

        Self {
            kind,
            left,
            right,
            condition,
            output_columns,
        }
    }
}

impl LogicalNode for JoinNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}

/// Extract table name from a PlanNode (if it's a TableScan)
fn extract_table_name_from_plan(plan: &dyn PlanNode) -> Option<String> {
    use crate::planner::nodes::table_scan::TableScanNode;
    crate::match_plan!(plan, {
        ts: TableScanNode => Some(ts.table.clone()),
        join: JoinNode => extract_table_name_from_plan(join.left.as_ref()),
        _ => None
    })
}

/// Check FK guarantee using plan nodes
fn check_fk_guarantee(
    join_kind: JoinKind,
    left: &dyn PlanNode,
    right: &dyn PlanNode,
    condition: &Option<JoinCondition>,
    schema: &Schema,
) -> bool {
    let left_table = extract_table_name_from_plan(left);
    let right_table = extract_table_name_from_plan(right);

    match join_kind {
        JoinKind::Left => {
            // Check if LEFT table has FK to RIGHT table
            if let (Some(left_tbl), Some(right_tbl)) = (left_table, right_table) {
                if let Some(table_def) = schema.tables.get(&left_tbl) {
                    for fk in &table_def.foreign_keys {
                        if fk.ref_table == right_tbl {
                            // Check if FK columns are NOT NULL
                            let fk_cols_not_null = fk.columns.iter().all(|fk_col| {
                                table_def
                                    .columns
                                    .iter()
                                    .any(|col| col.name == *fk_col && !col.nullable)
                            });

                            if fk_cols_not_null && matches!(condition, Some(JoinCondition::On(_))) {
                                return true;
                            }
                        }
                    }
                }
            }
        },
        JoinKind::Right => {
            // Check if RIGHT table has FK to LEFT table
            if let (Some(left_tbl), Some(right_tbl)) = (left_table, right_table) {
                if let Some(table_def) = schema.tables.get(&right_tbl) {
                    for fk in &table_def.foreign_keys {
                        if fk.ref_table == left_tbl {
                            let fk_cols_not_null = fk.columns.iter().all(|fk_col| {
                                table_def
                                    .columns
                                    .iter()
                                    .any(|col| col.name == *fk_col && !col.nullable)
                            });

                            if fk_cols_not_null && matches!(condition, Some(JoinCondition::On(_))) {
                                return true;
                            }
                        }
                    }
                }
            }
        },
        _ => {},
    }
    false
}
