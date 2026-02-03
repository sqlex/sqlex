use crate::{
    planner::plan::{
        AggregateExpr, CTEContext, GroupingMode, LogicalNode, PlanNode, PlanNodeColumn, TypedExpr,
        aggregate_result_type,
    },
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct AggregateNode {
    pub input: Box<dyn PlanNode>,
    pub group_by: Vec<TypedExpr>,
    pub aggregates: Vec<AggregateExpr>,
    pub grouping_mode: Option<GroupingMode>,
}

impl LogicalNode for AggregateNode {
    fn columns(&self, _schema: &Schema, _ctx: &CTEContext) -> Vec<PlanNodeColumn> {
        let mut cols: Vec<PlanNodeColumn> = self
            .group_by
            .iter()
            .enumerate()
            .map(|(i, expr)| PlanNodeColumn {
                name: format!("group_{}", i),
                data_type: expr.data_type.clone(),
                nullability: expr.nullable,
                origin_table: None,
                origin_column: None,
            })
            .collect();

        for (i, agg) in self.aggregates.iter().enumerate() {
            let (data_type, _nullable) = aggregate_result_type(&agg.function, &agg.args);
            cols.push(PlanNodeColumn {
                name: format!("agg_{}", i),
                data_type,
                nullability: false,
                origin_table: None,
                origin_column: None,
            });
        }

        cols
    }
}
