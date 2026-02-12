use super::Algebraizer;
use crate::ir::{auxiliary::ProjectionColumn, relational::RelationalExpr, scalar::ScalarExpr};

impl<'a> Algebraizer<'a> {
    /// Infer output column names from a RelationalExpr tree.
    pub(super) fn output_names_for_expr(&self, expr: &RelationalExpr) -> Vec<String> {
        match expr {
            RelationalExpr::Projection { columns, .. } => columns
                .iter()
                .enumerate()
                .map(|(idx, col)| output_name_for_projection(col, idx))
                .collect(),
            RelationalExpr::SetOperation { left, .. } => {
                // Set operations inherit names from the left side
                self.output_names_for_expr(left)
            },
            RelationalExpr::Sort { input, .. }
            | RelationalExpr::Limit { input, .. }
            | RelationalExpr::Distinct { input, .. }
            | RelationalExpr::Selection { input, .. }
            | RelationalExpr::Window { input, .. } => self.output_names_for_expr(input),
            RelationalExpr::Alias {
                input,
                column_aliases,
                ..
            } => {
                let names = self.output_names_for_expr(input);
                match column_aliases {
                    Some(alias_names) if alias_names.len() == names.len() => alias_names.clone(),
                    _ => names,
                }
            },
            RelationalExpr::Aggregation {
                group_by,
                aggregates,
                ..
            } => {
                let mut names = Vec::new();
                for (idx, expr) in group_by.iter().enumerate() {
                    names.push(name_from_scalar_expr(expr, idx));
                }
                for agg in aggregates {
                    names.push(agg.alias.clone());
                }
                names
            },
            RelationalExpr::Values { rows } => {
                let cols = rows.first().map(|r| r.len()).unwrap_or(0);
                (1..=cols).map(|i| format!("column{i}")).collect()
            },
            RelationalExpr::Scan { table, alias, .. } => {
                let table_name = alias.as_ref().unwrap_or(table);
                vec![table_name.clone()]
            },
            RelationalExpr::Join { left, right, .. } => {
                let mut names = self.output_names_for_expr(left);
                names.extend(self.output_names_for_expr(right));
                names
            },
        }
    }
}

fn output_name_for_projection(col: &ProjectionColumn, idx: usize) -> String {
    if let Some(alias) = &col.alias {
        return alias.clone();
    }
    name_from_scalar_expr(&col.expr, idx)
}

fn name_from_scalar_expr(expr: &ScalarExpr, idx: usize) -> String {
    match expr {
        ScalarExpr::ColumnRef { column, .. } => column.clone(),
        ScalarExpr::Function { name, .. } => name.clone(),
        ScalarExpr::AggregateCall { name, .. } => name.clone(),
        ScalarExpr::WindowCall { name, .. } => name.clone(),
        _ => format!("col_{idx}"),
    }
}
