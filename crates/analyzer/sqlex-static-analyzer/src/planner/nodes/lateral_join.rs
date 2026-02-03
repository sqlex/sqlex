use crate::{
    planner::plan::{CTEContext, JoinKind, LogicalNode, PlanNode, PlanNodeColumn},
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct LateralJoinNode {
    pub left: Box<dyn PlanNode>,
    pub lateral: Box<dyn PlanNode>,
    pub kind: JoinKind,
}

impl LogicalNode for LateralJoinNode {
    fn columns(&self, schema: &Schema, ctx: &CTEContext) -> Vec<PlanNodeColumn> {
        let mut left_cols = self.left.columns(schema, ctx);
        let mut right_cols = self.lateral.columns(schema, ctx);

        if matches!(self.kind, JoinKind::Left) {
            for col in &mut right_cols {
                col.nullability = true;
            }
        }

        left_cols.extend(right_cols);
        left_cols
    }
}
