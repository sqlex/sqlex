use crate::{
    analysis::bind::Binder,
    ir::bound::{BoundExpr, BoundQuery, BoundSelect, BoundSetExpr},
};

impl<'a> Binder<'a> {
    pub(super) fn output_names_for_query(&self, query: &BoundQuery) -> Vec<String> {
        match &query.body {
            BoundSetExpr::Select(select) => self.output_names_for_select(query, select),
            BoundSetExpr::SetOperation { left, .. } => self.output_names_for_setexpr(query, left),
            BoundSetExpr::Query(subquery) => self.output_names_for_query(subquery),
            BoundSetExpr::Values { rows } => {
                let cols = rows.first().map(|r| r.len()).unwrap_or(0);
                (1..=cols).map(|i| format!("column{}", i)).collect()
            },
            _ => Vec::new(),
        }
    }

    fn output_names_for_setexpr(&self, query: &BoundQuery, expr: &BoundSetExpr) -> Vec<String> {
        match expr {
            BoundSetExpr::Select(select) => self.output_names_for_select(query, select),
            BoundSetExpr::SetOperation { left, .. } => self.output_names_for_setexpr(query, left),
            BoundSetExpr::Query(query) => self.output_names_for_query(query),
            BoundSetExpr::Values { rows } => {
                let cols = rows.first().map(|r| r.len()).unwrap_or(0);
                (1..=cols).map(|i| format!("column{}", i)).collect()
            },
            _ => Vec::new(),
        }
    }

    fn output_names_for_select(&self, query: &BoundQuery, select: &BoundSelect) -> Vec<String> {
        let mut names = Vec::new();
        for (idx, proj) in select.projection.iter().enumerate() {
            if let Some(alias) = &proj.alias {
                names.push(alias.clone());
                continue;
            }
            let name = match query.exprs.get(proj.expr) {
                BoundExpr::Column(column_id) => query.columns.get(*column_id).name.clone(),
                _ => format!("col_{}", idx),
            };
            names.push(name);
        }
        names
    }
}
