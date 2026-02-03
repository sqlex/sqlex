use crate::planner::{
    expr::{AggregateExpr, TypedExpr},
    plan::{LogicalNode, PlanNode, PlanNodeColumn},
};

/// Grouping mode for advanced GROUP BY
#[derive(Debug, Clone)]
pub enum GroupingMode {
    GroupingSets(Vec<Vec<TypedExpr>>),
    Cube,
    Rollup,
}

#[derive(Debug, Clone)]
pub struct AggregateNode {
    pub input: Box<dyn PlanNode>,
    pub group_by: Vec<TypedExpr>,
    pub aggregates: Vec<AggregateExpr>,
    pub grouping_mode: Option<GroupingMode>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl AggregateNode {
    pub fn build(
        input: Box<dyn PlanNode>,
        group_by: Vec<TypedExpr>,
        aggregates: Vec<AggregateExpr>,
        grouping_mode: Option<GroupingMode>,
    ) -> Self {
        let mut output_columns: Vec<PlanNodeColumn> = group_by
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

        for (i, agg) in aggregates.iter().enumerate() {
            let (data_type, _nullable) = agg.function.result_type(&agg.args);
            output_columns.push(PlanNodeColumn {
                name: format!("agg_{}", i),
                data_type,
                nullability: false, // Aggregates usually handle nulls or produce specific types. Keeping logic same as before.
                origin_table: None,
                origin_column: None,
            });
        }

        Self {
            input,
            group_by,
            aggregates,
            grouping_mode,
            output_columns,
        }
    }
}

impl LogicalNode for AggregateNode {
    fn columns(&self) -> &[PlanNodeColumn] {
        &self.output_columns
    }
}
