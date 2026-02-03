use crate::{
    planner::{
        nullability::{self, ColumnNullability},
        plan::{JoinCondition, JoinKind, LogicalNode, PlanNode, PlanNodeColumn},
    },
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct JoinNode {
    pub kind: JoinKind,
    pub left: Box<dyn PlanNode>,
    pub right: Box<dyn PlanNode>,
    pub condition: Option<JoinCondition>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl JoinNode {
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
        let nullability_result =
            nullability::join_nullability(kind, left.as_ref(), right.as_ref(), &condition, schema);

        // Apply nullability to left side tables if needed
        if let ColumnNullability::ForceNullable(side) = nullability_result {
            if side == nullability::JoinSide::Left || side == nullability::JoinSide::Both {
                for col in &mut left_cols {
                    col.nullability = true;
                }
            }
        }

        // Apply nullability to right side tables if needed
        let make_right_nullable = match nullability_result {
            ColumnNullability::ForceNullable(side) => {
                side == nullability::JoinSide::Right || side == nullability::JoinSide::Both
            },
            _ => false,
        };

        if make_right_nullable {
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
