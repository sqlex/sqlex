use sqlex_analyzer::extension::DataTypeExt;
use sqlex_common::types::{Cardinality, DataType};

use super::{ColumnMetadata, Inferrer, RelationalMetadata};
use crate::{
    diagnostics::Diagnostic,
    ir::{
        auxiliary::{BinaryOp, JoinCondition, JoinKind, SetOp},
        relational::RelationalExpr,
        scalar::{LiteralValue, ScalarExpr},
    },
};

impl Inferrer<'_> {
    /// Infer metadata for a relational expression by dispatching to per-operator rules.
    pub(super) fn infer_expr(&mut self, expr: &RelationalExpr) -> RelationalMetadata {
        match expr {
            RelationalExpr::Scan { table, alias } => self.infer_scan(table, alias.as_deref()),
            RelationalExpr::Values { rows } => self.infer_values(rows),
            RelationalExpr::Selection { input, condition } => {
                self.infer_selection(input, condition)
            },
            RelationalExpr::Projection { input, columns } => self.infer_projection(input, columns),
            RelationalExpr::Aggregation {
                input, group_by, ..
            } => self.infer_aggregation(input, group_by),
            RelationalExpr::Window { input, .. } => self.infer_window(input),
            RelationalExpr::Distinct { input } => self.infer_distinct(input),
            RelationalExpr::Sort { input, .. } => self.infer_sort(input),
            RelationalExpr::Limit {
                input,
                count,
                offset,
            } => self.infer_limit(input, count.as_ref(), offset.as_ref()),
            RelationalExpr::Join {
                left,
                right,
                kind,
                condition,
            } => self.infer_join(left, right, *kind, condition.as_ref()),
            RelationalExpr::SetOperation {
                left, right, op, ..
            } => self.infer_set_operation(left, right, *op),
            RelationalExpr::Alias {
                input,
                name,
                column_aliases,
            } => self.infer_alias(input, name, column_aliases.as_deref()),
        }
    }

    fn infer_scan(&self, table: &str, alias: Option<&str>) -> RelationalMetadata {
        let table_label = alias.unwrap_or(table).to_string();
        let columns = match self.catalog.get_table(table) {
            Some(table_def) => table_def
                .columns
                .iter()
                .map(|col| ColumnMetadata {
                    table: Some(table_label.clone()),
                    name: col.name.clone(),
                    data_type: col.data_type.clone(),
                    nullable: col.nullable,
                })
                .collect(),
            None => Vec::new(),
        };
        RelationalMetadata {
            columns,
            cardinality: Cardinality::Unknown,
        }
    }

    fn infer_values(&mut self, rows: &[Vec<ScalarExpr>]) -> RelationalMetadata {
        let Some(first_row) = rows.first() else {
            return RelationalMetadata {
                columns: Vec::new(),
                cardinality: Cardinality::AtMostOne,
            };
        };

        let col_count = first_row.len();
        let mut types: Vec<Option<DataType>> = Vec::with_capacity(col_count);
        let mut nullables: Vec<bool> = Vec::with_capacity(col_count);

        for expr in first_row {
            let info = self.infer_scalar(expr, &[]);
            types.push(Some(info.data_type));
            nullables.push(info.nullable);
        }

        for row in rows.iter().skip(1) {
            for (i, expr) in row.iter().enumerate() {
                if i < col_count {
                    let info = self.infer_scalar(expr, &[]);
                    types[i] =
                        DataType::merge_common_type(self.dialect, types[i].take(), &info.data_type);
                    if info.nullable {
                        nullables[i] = true;
                    }
                }
            }
        }

        let columns = types
            .into_iter()
            .zip(nullables)
            .enumerate()
            .map(|(i, (dt, nullable))| ColumnMetadata {
                table: None,
                name: format!("column{}", i + 1),
                data_type: dt.unwrap_or_else(|| DataType::Custom("unknown".to_string())),
                nullable,
            })
            .collect();

        let cardinality = match rows.len() {
            0 => Cardinality::AtMostOne,
            1 => Cardinality::ExactlyOne,
            _ => Cardinality::AtLeastOne,
        };

        RelationalMetadata {
            columns,
            cardinality,
        }
    }

    fn infer_set_operation(
        &mut self,
        left: &RelationalExpr,
        right: &RelationalExpr,
        op: SetOp,
    ) -> RelationalMetadata {
        let left_meta = self.infer_expr(left);
        let right_meta = self.infer_expr(right);

        if left_meta.columns.len() != right_meta.columns.len() {
            self.diagnostics
                .push(Diagnostic::set_operation_column_count_mismatch());
            return left_meta;
        }

        let columns = left_meta
            .columns
            .into_iter()
            .zip(right_meta.columns)
            .map(|(l, r)| {
                let data_type =
                    DataType::merge_common_type(self.dialect, Some(l.data_type), &r.data_type)
                        .unwrap_or_else(|| DataType::Custom("unknown".to_string()));
                ColumnMetadata {
                    table: None,
                    name: l.name,
                    data_type,
                    nullable: l.nullable || r.nullable,
                }
            })
            .collect();

        let cardinality = match op {
            SetOp::Union => {
                super::cardinality::max_cardinality(left_meta.cardinality, right_meta.cardinality)
            },
            SetOp::Intersect => {
                super::cardinality::min_cardinality(left_meta.cardinality, right_meta.cardinality)
            },
            SetOp::Except => left_meta.cardinality,
        };

        RelationalMetadata {
            columns,
            cardinality,
        }
    }

    fn infer_selection(
        &mut self,
        input: &RelationalExpr,
        condition: &ScalarExpr,
    ) -> RelationalMetadata {
        let mut meta = self.infer_expr(input);
        // Analyze the WHERE condition for cardinality refinement
        if let Some(refined) = self.analyze_selection_cardinality(input, condition) {
            meta.cardinality = refined;
        }
        meta
    }

    fn infer_projection(
        &mut self,
        input: &RelationalExpr,
        columns: &[crate::ir::auxiliary::ProjectionColumn],
    ) -> RelationalMetadata {
        let input_meta = self.infer_expr(input);

        let mut result_columns = Vec::new();
        for (idx, proj) in columns.iter().enumerate() {
            match &proj.expr {
                ScalarExpr::Wildcard => {
                    // Expand wildcard to all input columns
                    for col in &input_meta.columns {
                        result_columns.push(col.clone());
                    }
                },
                ScalarExpr::QualifiedWildcard { table } => {
                    // Expand qualified wildcard to columns from the specified table
                    let expanded = self.expand_qualified_wildcard(input, table);
                    result_columns.extend(expanded);
                },
                expr => {
                    let info = self.infer_scalar(expr, &input_meta.columns);
                    let name = proj
                        .alias
                        .clone()
                        .unwrap_or_else(|| self.infer_scalar_name(expr, &input_meta.columns, idx));
                    result_columns.push(ColumnMetadata {
                        table: None,
                        name,
                        data_type: info.data_type,
                        nullable: info.nullable,
                    });
                },
            }
        }

        RelationalMetadata {
            columns: result_columns,
            cardinality: input_meta.cardinality,
        }
    }

    fn infer_aggregation(
        &mut self,
        input: &RelationalExpr,
        group_by: &[ScalarExpr],
    ) -> RelationalMetadata {
        let input_meta = self.infer_expr(input);

        // Validate GROUP BY expressions (triggers unknown column detection)
        for expr in group_by {
            self.infer_scalar(expr, &input_meta.columns);
        }

        // Aggregation without GROUP BY produces exactly one row
        let cardinality = if group_by.is_empty() {
            Cardinality::ExactlyOne
        } else {
            Cardinality::Unknown
        };

        // The aggregation node itself doesn't define output columns —
        // those come from the Projection above it. Pass through input columns
        // so the Projection can resolve column references.
        RelationalMetadata {
            columns: input_meta.columns,
            cardinality,
        }
    }

    fn infer_window(&mut self, input: &RelationalExpr) -> RelationalMetadata {
        // Window functions don't change the number of rows or the base schema.
        // Additional window columns are handled in the Projection above.
        self.infer_expr(input)
    }

    fn infer_distinct(&mut self, input: &RelationalExpr) -> RelationalMetadata {
        let mut meta = self.infer_expr(input);
        meta.cardinality = super::cardinality::drop_lower_bound(meta.cardinality);
        meta
    }

    fn infer_sort(&mut self, input: &RelationalExpr) -> RelationalMetadata {
        // Sort doesn't change schema or cardinality
        self.infer_expr(input)
    }

    fn infer_alias(
        &mut self,
        input: &RelationalExpr,
        name: &str,
        column_aliases: Option<&[String]>,
    ) -> RelationalMetadata {
        let mut meta = self.infer_expr(input);
        // Re-label all output columns with the alias name
        for col in &mut meta.columns {
            col.table = Some(name.to_string());
        }
        if let Some(alias_names) = column_aliases {
            if alias_names.len() == meta.columns.len() {
                for (col, alias_name) in meta.columns.iter_mut().zip(alias_names.iter()) {
                    col.name = alias_name.clone();
                }
            }
        }
        meta
    }

    fn infer_limit(
        &mut self,
        input: &RelationalExpr,
        count: Option<&ScalarExpr>,
        offset: Option<&ScalarExpr>,
    ) -> RelationalMetadata {
        let mut meta = self.infer_expr(input);

        if let Some(limit_val) = count.and_then(scalar_to_u64) {
            if limit_val == 0 {
                meta.cardinality = Cardinality::AtMostOne;
            } else if limit_val == 1 {
                meta.cardinality = super::cardinality::constrain_at_most_one(meta.cardinality);
            }
        }

        if let Some(offset_val) = offset.and_then(scalar_to_u64) {
            if offset_val > 0 {
                meta.cardinality = super::cardinality::drop_lower_bound(meta.cardinality);
            }
        }

        meta
    }

    fn infer_join(
        &mut self,
        left: &RelationalExpr,
        right: &RelationalExpr,
        kind: JoinKind,
        condition: Option<&JoinCondition>,
    ) -> RelationalMetadata {
        let left_meta = self.infer_expr(left);
        let right_meta = self.infer_expr(right);

        // Collect USING column names for deduplication
        let using_cols: Vec<String> = match condition {
            Some(JoinCondition::Using(cols)) => cols.iter().map(|c| c.to_lowercase()).collect(),
            _ => Vec::new(),
        };

        let mut columns = Vec::new();

        // Left columns: nullable if RIGHT or FULL join
        let left_nullable = matches!(kind, JoinKind::Right | JoinKind::Full);
        for col in &left_meta.columns {
            let mut c = col.clone();
            if left_nullable && !self.left_join_preserves(left, right, condition, kind) {
                c.nullable = true;
            }
            columns.push(c);
        }

        // Right columns: nullable if LEFT or FULL join
        // Skip USING columns from the right side (they are coalesced with the left)
        let right_nullable = matches!(kind, JoinKind::Left | JoinKind::Full);
        for col in &right_meta.columns {
            if !using_cols.is_empty()
                && using_cols.iter().any(|u| u.eq_ignore_ascii_case(&col.name))
            {
                continue;
            }
            let mut c = col.clone();
            if right_nullable && !self.right_join_preserves(left, right, condition, kind) {
                c.nullable = true;
            }
            columns.push(c);
        }

        let cardinality = super::cardinality::combine_join_cardinality(
            left_meta.cardinality,
            right_meta.cardinality,
            kind,
        );

        RelationalMetadata {
            columns,
            cardinality,
        }
    }

    // ------------------------------------------------------------------
    // Qualified wildcard expansion
    // ------------------------------------------------------------------

    /// Expand `t.*` by walking the input tree to find the Scan for `table`
    /// and returning its catalog columns (with join-induced nullability).
    pub(super) fn expand_qualified_wildcard(
        &self,
        input: &RelationalExpr,
        table: &str,
    ) -> Vec<ColumnMetadata> {
        let real_table = find_scan_table(input, table);
        let Some(real_table) = real_table else {
            return Vec::new();
        };

        match self.catalog.get_table(&real_table) {
            Some(table_def) => table_def
                .columns
                .iter()
                .map(|col| ColumnMetadata {
                    table: Some(table.to_string()),
                    name: col.name.clone(),
                    data_type: col.data_type.clone(),
                    nullable: col.nullable,
                })
                .collect(),
            None => Vec::new(),
        }
    }

    // ------------------------------------------------------------------
    // FK-based join nullability preservation
    // ------------------------------------------------------------------

    fn left_join_preserves(
        &self,
        left: &RelationalExpr,
        right: &RelationalExpr,
        condition: Option<&JoinCondition>,
        _kind: JoinKind,
    ) -> bool {
        let Some(condition) = condition else {
            return false;
        };
        let Some((left_col, right_col)) = extract_eq_join_columns(condition) else {
            return false;
        };
        let right_table = find_scan_table_for_column(right);
        let left_table = find_scan_table_for_column(left);
        let (Some(rt), Some(lt)) = (right_table, left_table) else {
            return false;
        };
        self.fk_guarantees_match(&rt, &right_col, &lt, &left_col)
    }

    fn right_join_preserves(
        &self,
        left: &RelationalExpr,
        right: &RelationalExpr,
        condition: Option<&JoinCondition>,
        _kind: JoinKind,
    ) -> bool {
        let Some(condition) = condition else {
            return false;
        };
        let Some((left_col, right_col)) = extract_eq_join_columns(condition) else {
            return false;
        };
        let left_table = find_scan_table_for_column(left);
        let right_table = find_scan_table_for_column(right);
        let (Some(lt), Some(rt)) = (left_table, right_table) else {
            return false;
        };
        self.fk_guarantees_match(&lt, &left_col, &rt, &right_col)
    }

    fn fk_guarantees_match(
        &self,
        fk_table: &str,
        fk_col: &str,
        pk_table: &str,
        pk_col: &str,
    ) -> bool {
        let Some(fk_table_def) = self.catalog.get_table(fk_table) else {
            return false;
        };
        let fk_col_nullable = fk_table_def
            .get_column(fk_col)
            .map(|c| c.nullable)
            .unwrap_or(true);
        if fk_col_nullable {
            return false;
        }
        for fk in &fk_table_def.foreign_keys {
            if !fk.ref_table.eq_ignore_ascii_case(pk_table) {
                continue;
            }
            if fk.columns.len() == 1
                && fk.ref_columns.len() == 1
                && fk.columns[0].eq_ignore_ascii_case(fk_col)
                && fk.ref_columns[0].eq_ignore_ascii_case(pk_col)
            {
                return true;
            }
        }
        false
    }

    // ------------------------------------------------------------------
    // Expression name inference
    // ------------------------------------------------------------------

    /// Infer a column name from a scalar expression when no alias is provided.
    pub(super) fn infer_scalar_name(
        &self,
        expr: &ScalarExpr,
        input_columns: &[ColumnMetadata],
        _idx: usize,
    ) -> String {
        self.format_scalar_name(expr, input_columns, 0)
    }

    fn format_scalar_name(
        &self,
        expr: &ScalarExpr,
        input_columns: &[ColumnMetadata],
        level: usize,
    ) -> String {
        match expr {
            ScalarExpr::ColumnRef { table, column } => {
                if level == 0 {
                    return column.clone();
                }
                // MySQL and SQLite include table qualifier in nested refs
                if matches!(
                    self.dialect,
                    sqlex_common::dialect::Dialect::MySQL | sqlex_common::dialect::Dialect::SQLite
                ) {
                    if let Some(t) = table {
                        return format!("{t}.{column}");
                    }
                }
                column.clone()
            },
            ScalarExpr::Literal(lit) => self.format_literal_name(lit, level),
            ScalarExpr::BinaryOp { left, op, right } => {
                if matches!(self.dialect, sqlex_common::dialect::Dialect::Postgres) {
                    return "?column?".to_string();
                }
                let l = self.format_scalar_name(left, input_columns, level + 1);
                let r = self.format_scalar_name(right, input_columns, level + 1);
                let op_str = format_binary_op_str(*op);
                format!("{l} {op_str} {r}")
            },
            ScalarExpr::UnaryOp { op, expr } => {
                if matches!(self.dialect, sqlex_common::dialect::Dialect::Postgres) {
                    return "?column?".to_string();
                }
                let inner = self.format_scalar_name(expr, input_columns, level + 1);
                let op_str = format_unary_op_str(*op);
                format!("{op_str}{inner}")
            },
            ScalarExpr::Function { name, args } | ScalarExpr::AggregateCall { name, args, .. } => {
                if matches!(self.dialect, sqlex_common::dialect::Dialect::Postgres) {
                    return name.to_lowercase();
                }
                let args_str = args
                    .iter()
                    .map(|a| self.format_scalar_name(a, input_columns, level + 1))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{name}({args_str})")
            },
            ScalarExpr::WindowCall { name, args, .. } => {
                if matches!(self.dialect, sqlex_common::dialect::Dialect::Postgres) {
                    return name.to_lowercase();
                }
                let args_str = args
                    .iter()
                    .map(|a| self.format_scalar_name(a, input_columns, level + 1))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{name}({args_str})")
            },
            ScalarExpr::IsNull { expr, negated } => {
                if matches!(self.dialect, sqlex_common::dialect::Dialect::Postgres) {
                    return "?column?".to_string();
                }
                let inner = self.format_scalar_name(expr, input_columns, level + 1);
                if *negated {
                    format!("{inner} IS NOT NULL")
                } else {
                    format!("{inner} IS NULL")
                }
            },
            ScalarExpr::Case { .. } => {
                if matches!(self.dialect, sqlex_common::dialect::Dialect::Postgres) {
                    return "case".to_string();
                }
                self.format_case_name(expr, input_columns, level)
            },
            ScalarExpr::Cast { expr, .. } => self.format_scalar_name(expr, input_columns, level),
            ScalarExpr::Wildcard => "*".to_string(),
            _ => {
                if matches!(self.dialect, sqlex_common::dialect::Dialect::Postgres) {
                    "?column?".to_string()
                } else {
                    "?".to_string()
                }
            },
        }
    }

    fn format_literal_name(&self, lit: &LiteralValue, level: usize) -> String {
        match self.dialect {
            sqlex_common::dialect::Dialect::Postgres => "?column?".to_string(),
            _ => {
                let formatted = match lit {
                    LiteralValue::Null => "NULL".to_string(),
                    LiteralValue::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
                    LiteralValue::Integer(n) => n.to_string(),
                    LiteralValue::Float(f) => format!("{f}"),
                    LiteralValue::String(s) => format!("'{s}'"),
                };
                // Remove quotes for top-level string literals in MySQL
                if level == 0
                    && matches!(self.dialect, sqlex_common::dialect::Dialect::MySQL)
                    && matches!(lit, LiteralValue::String(_))
                    && formatted.starts_with('\'')
                    && formatted.ends_with('\'')
                {
                    return formatted[1..formatted.len() - 1].to_string();
                }
                formatted
            },
        }
    }

    fn format_case_name(
        &self,
        expr: &ScalarExpr,
        input_columns: &[ColumnMetadata],
        level: usize,
    ) -> String {
        let ScalarExpr::Case {
            operand,
            when_clauses,
            else_result,
        } = expr
        else {
            return "?".to_string();
        };

        let mut parts = vec!["CASE".to_string()];
        if let Some(op) = operand {
            parts.push(self.format_scalar_name(op, input_columns, level + 1));
        }
        for clause in when_clauses {
            parts.push(format!(
                "WHEN {}",
                self.format_scalar_name(&clause.condition, input_columns, level + 1)
            ));
            parts.push(format!(
                "THEN {}",
                self.format_scalar_name(&clause.result, input_columns, level + 1)
            ));
        }
        if let Some(el) = else_result {
            parts.push(format!(
                "ELSE {}",
                self.format_scalar_name(el, input_columns, level + 1)
            ));
        }
        parts.push("END".to_string());
        parts.join(" ")
    }
}

// ------------------------------------------------------------------
// Free functions
// ------------------------------------------------------------------

fn scalar_to_u64(expr: &ScalarExpr) -> Option<u64> {
    if let ScalarExpr::Literal(LiteralValue::Integer(n)) = expr {
        u64::try_from(*n).ok()
    } else {
        None
    }
}

/// Find the real table name for a Scan node whose table or alias matches `name`.
fn find_scan_table(expr: &RelationalExpr, name: &str) -> Option<String> {
    match expr {
        RelationalExpr::Scan { table, alias } => {
            if alias
                .as_deref()
                .map(|a| a.eq_ignore_ascii_case(name))
                .unwrap_or(false)
            {
                return Some(table.clone());
            }
            if table.eq_ignore_ascii_case(name) {
                return Some(table.clone());
            }
            None
        },
        RelationalExpr::Alias {
            input, name: alias, ..
        } => {
            if alias.eq_ignore_ascii_case(name) {
                // The alias itself matches — return the alias as the "table"
                // (derived tables don't have a real catalog table)
                return Some(alias.clone());
            }
            find_scan_table(input, name)
        },
        RelationalExpr::Selection { input, .. }
        | RelationalExpr::Projection { input, .. }
        | RelationalExpr::Aggregation { input, .. }
        | RelationalExpr::Window { input, .. }
        | RelationalExpr::Distinct { input }
        | RelationalExpr::Sort { input, .. }
        | RelationalExpr::Limit { input, .. } => find_scan_table(input, name),
        RelationalExpr::Join { left, right, .. }
        | RelationalExpr::SetOperation { left, right, .. } => {
            find_scan_table(left, name).or_else(|| find_scan_table(right, name))
        },
        RelationalExpr::Values { .. } => None,
    }
}

/// Find the real table name for a column reference within an expression tree.
/// Looks for the first Scan node in the tree.
fn find_scan_table_for_column(expr: &RelationalExpr) -> Option<String> {
    match expr {
        RelationalExpr::Scan { table, .. } => Some(table.clone()),
        RelationalExpr::Alias { input, .. }
        | RelationalExpr::Selection { input, .. }
        | RelationalExpr::Projection { input, .. }
        | RelationalExpr::Aggregation { input, .. }
        | RelationalExpr::Window { input, .. }
        | RelationalExpr::Distinct { input }
        | RelationalExpr::Sort { input, .. }
        | RelationalExpr::Limit { input, .. } => find_scan_table_for_column(input),
        RelationalExpr::Join { left, right, .. } => {
            find_scan_table_for_column(left).or_else(|| find_scan_table_for_column(right))
        },
        _ => None,
    }
}

/// Extract the left and right column names from an equality join condition.
fn extract_eq_join_columns(condition: &JoinCondition) -> Option<(String, String)> {
    match condition {
        JoinCondition::On(expr) => {
            if let ScalarExpr::BinaryOp {
                left,
                op: BinaryOp::Eq,
                right,
            } = expr
            {
                if let (
                    ScalarExpr::ColumnRef { column: l, .. },
                    ScalarExpr::ColumnRef { column: r, .. },
                ) = (left.as_ref(), right.as_ref())
                {
                    return Some((l.clone(), r.clone()));
                }
            }
            None
        },
        JoinCondition::Using(_) | JoinCondition::Natural => None,
    }
}

fn format_binary_op_str(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Eq => "=",
        BinaryOp::NotEq => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::LtEq => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::GtEq => ">=",
        BinaryOp::And => "AND",
        BinaryOp::Or => "OR",
        BinaryOp::Xor => "XOR",
        BinaryOp::Like => "LIKE",
        BinaryOp::NotLike => "NOT LIKE",
        BinaryOp::StringConcat => "||",
        _ => "?",
    }
}

fn format_unary_op_str(op: crate::ir::auxiliary::UnaryOp) -> &'static str {
    use crate::ir::auxiliary::UnaryOp;
    match op {
        UnaryOp::Plus => "+",
        UnaryOp::Neg => "-",
        UnaryOp::Not => "NOT ",
    }
}
