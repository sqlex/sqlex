use crate::{
    planner::plan::{CTEContext, CTEDef, LogicalNode, PlanNode, PlanNodeColumn},
    schema::Schema,
};

#[derive(Debug, Clone)]
pub struct WithCTENode {
    pub ctes: Vec<CTEDef>,
    pub body: Box<dyn PlanNode>,
}

impl LogicalNode for WithCTENode {
    fn columns(&self, schema: &Schema, ctx: &CTEContext) -> Vec<PlanNodeColumn> {
        let mut new_ctx = ctx.clone();
        for cte in &self.ctes {
            let cte_cols = cte.query.columns(schema, &new_ctx);
            // Apply column aliases if specified
            let final_cols = if let Some(ref aliases) = cte.columns {
                cte_cols
                    .into_iter()
                    .zip(aliases.iter())
                    .map(|(mut c, alias)| {
                        c.name = alias.clone();
                        c
                    })
                    .collect()
            } else {
                cte_cols
            };
            new_ctx.insert(cte.name.clone(), final_cols);
        }
        self.body.columns(schema, &new_ctx)
    }
}
