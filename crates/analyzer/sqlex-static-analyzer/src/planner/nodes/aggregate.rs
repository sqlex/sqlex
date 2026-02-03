use crate::planner::{
    expr::{AggregateFunctionExpr, Expression},
    plan::{LogicalNode, PlanNode, PlanNodeColumn},
};

/// Grouping mode for advanced GROUP BY
#[derive(Debug, Clone)]
pub enum GroupingMode {
    GroupingSets(Vec<Vec<Box<dyn Expression>>>),
    Cube,
    Rollup,
}

#[derive(Debug, Clone)]
pub struct AggregateNode {
    pub input: Box<dyn PlanNode>,
    pub group_by: Vec<Box<dyn Expression>>,
    pub aggregates: Vec<AggregateFunctionExpr>,
    pub grouping_mode: Option<GroupingMode>,
    pub output_columns: Vec<PlanNodeColumn>,
}

impl AggregateNode {
    pub fn build(
        input: Box<dyn PlanNode>,
        group_by: Vec<Box<dyn Expression>>,
        aggregates: Vec<AggregateFunctionExpr>,
        grouping_mode: Option<GroupingMode>,
    ) -> Self {
        let mut output_columns: Vec<PlanNodeColumn> = group_by
            .iter()
            .enumerate()
            .map(|(i, expr)| PlanNodeColumn {
                name: format!("group_{}", i),
                data_type: expr.data_type(),
                nullability: expr.nullable(),
                origin_table: None,
                origin_column: None,
            })
            .collect();

        for (i, agg) in aggregates.iter().enumerate() {
            let name = format!("agg_{}", i);
            let (data_type, nullable) = (agg.return_type.clone(), agg.is_nullable);

            output_columns.push(PlanNodeColumn {
                name,
                data_type,
                nullability: nullable,
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
