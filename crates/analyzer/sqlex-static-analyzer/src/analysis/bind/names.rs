use crate::{
    analysis::bind::Binder,
    ir::bound::{BoundExpr, BoundQueryBody, BoundSelect, BoundSetExpr},
};

impl<'a> Binder<'a> {
    pub(super) fn output_names_for_query_body(&self, query: &BoundQueryBody) -> Vec<String> {
        self.output_names_for_setexpr(&query.body)
    }

    fn output_names_for_setexpr(&self, expr: &BoundSetExpr) -> Vec<String> {
        match expr {
            BoundSetExpr::Select(select) => self.output_names_for_select(select),
            BoundSetExpr::SetOperation { left, .. } => self.output_names_for_setexpr(left),
            BoundSetExpr::Query(subquery) => self.output_names_for_query_body(subquery),
            BoundSetExpr::Values { rows } => {
                let cols = rows.first().map(|r| r.len()).unwrap_or(0);
                (1..=cols).map(|i| format!("column{}", i)).collect()
            },
        }
    }

    fn output_names_for_select(&self, select: &BoundSelect) -> Vec<String> {
        let mut names = Vec::new();
        for (idx, proj) in select.projection.iter().enumerate() {
            if let Some(alias) = &proj.alias {
                names.push(alias.clone());
                continue;
            }
            let name = match self.exprs.get(proj.expr) {
                BoundExpr::Column(column_id) => self.columns.get(*column_id).name.clone(),
                _ => format!("col_{}", idx),
            };
            names.push(name);
        }
        names
    }
}
