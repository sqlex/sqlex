use sqlex_common::ColumnInfo;

use crate::{
    planner::plan::{CTEContext, JoinCondition, JoinKind, LogicalNode, PlanNode},
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct JoinNode {
    pub kind: JoinKind,
    pub left: Box<dyn PlanNode>,
    pub right: Box<dyn PlanNode>,
    pub condition: Option<JoinCondition>,
}

impl LogicalNode for JoinNode {
    fn columns(&self, schema: &Schema, ctx: &CTEContext) -> Vec<ColumnInfo> {
        let mut left_cols = self.left.columns(schema, ctx);
        let mut right_cols = self.right.columns(schema, ctx);

        // Apply nullability based on join type
        match self.kind {
            JoinKind::Left => {
                // Right side becomes nullable
                for col in &mut right_cols {
                    col.nullability = true;
                }
            },
            JoinKind::Right => {
                // Left side becomes nullable
                for col in &mut left_cols {
                    col.nullability = true;
                }
            },
            JoinKind::Full => {
                // Both sides become nullable
                for col in &mut left_cols {
                    col.nullability = true;
                }
                for col in &mut right_cols {
                    col.nullability = true;
                }
            },
            JoinKind::Inner | JoinKind::Cross => {
                // No nullability changes
            },
        }

        left_cols.extend(right_cols);
        left_cols
    }
}
