use sqlparser::ast::BinaryOperator;

use super::{QueryTypeState, TypeContext, TypeInfo};
use crate::ir::{BoundExpr, BoundQuery, BoundTableSource, ColumnId, ExprId};

impl<'a> TypeContext<'a> {
    pub(super) fn infer_expr(
        &mut self,
        state: &mut QueryTypeState<'_>,
        expr_id: ExprId,
    ) -> TypeInfo {
        if let Some(info) = state.types.get(&expr_id) {
            return info.clone();
        }

        let info = match state.query.exprs.get(expr_id) {
            BoundExpr::Column(column_id) => self.infer_column(state, *column_id),
            BoundExpr::Literal(value) => infer_literal(value),
            BoundExpr::Binary { left, op, right } => {
                let left_info = self.infer_expr(state, *left);
                let right_info = self.infer_expr(state, *right);
                TypeInfo {
                    data_type: analyze_binary_type(&left_info.data_type, op, &right_info.data_type),
                    nullable: left_info.nullable || right_info.nullable,
                }
            },
            BoundExpr::Unary { expr, .. } => self.infer_expr(state, *expr),
            BoundExpr::IsNull { .. } => TypeInfo {
                data_type: sqlex_common::DataType::Bool,
                nullable: false,
            },
            BoundExpr::Function {
                name,
                args,
                distinct,
                over,
            } => {
                let arg_types = args
                    .iter()
                    .map(|id| self.infer_expr(state, *id).data_type)
                    .collect::<Vec<_>>();
                let arg_nullables = args
                    .iter()
                    .map(|id| self.infer_expr(state, *id).nullable)
                    .collect::<Vec<_>>();

                self.infer_function(name, &arg_types, &arg_nullables, *distinct, *over)
            },
            BoundExpr::Case {
                operand,
                conditions,
                results,
                else_result,
            } => {
                if let Some(expr_id) = operand {
                    self.infer_expr(state, *expr_id);
                }
                for expr_id in conditions {
                    self.infer_expr(state, *expr_id);
                }

                let mut merged_type: Option<sqlex_common::DataType> = None;
                let mut nullable = else_result.is_none();

                for expr_id in results {
                    let info = self.infer_expr(state, *expr_id);
                    merged_type = merge_types(merged_type, info.data_type.clone());
                    if info.nullable {
                        nullable = true;
                    }
                }

                if let Some(expr_id) = else_result {
                    let info = self.infer_expr(state, *expr_id);
                    merged_type = merge_types(merged_type, info.data_type.clone());
                    if info.nullable {
                        nullable = true;
                    }
                }

                TypeInfo {
                    data_type: merged_type
                        .unwrap_or_else(|| sqlex_common::DataType::Custom("unknown".to_string())),
                    nullable,
                }
            },
            BoundExpr::Subquery(subquery) => {
                let schema = self.output_schema_for_query(subquery);
                if schema.columns.len() == 1 {
                    let col = &schema.columns[0];
                    TypeInfo {
                        data_type: col.data_type.clone(),
                        nullable: true,
                    }
                } else {
                    self.diagnostics.push(
                        super::super::diagnostics::Diagnostic::scalar_subquery_column_count(),
                    );
                    TypeInfo {
                        data_type: sqlex_common::DataType::Custom("unknown".to_string()),
                        nullable: true,
                    }
                }
            },
            BoundExpr::Unsupported => TypeInfo {
                data_type: sqlex_common::DataType::Custom("unknown".to_string()),
                nullable: true,
            },
        };

        state.types.insert(expr_id, info.clone());
        info
    }

    fn infer_column(&mut self, state: &QueryTypeState<'_>, column_id: ColumnId) -> TypeInfo {
        let column = state.query.columns.get(column_id);
        let table = state.query.tables.get(column.table);

        let mut info = match &table.source {
            BoundTableSource::Table { name } => match self.catalog.get_table(name) {
                Some(table_def) => match table_def.get_column(&column.name) {
                    Some(col_def) => TypeInfo {
                        data_type: col_def.data_type.clone(),
                        nullable: col_def.nullable,
                    },
                    None => {
                        self.diagnostics.push(
                            super::super::diagnostics::Diagnostic::unknown_column(&format!(
                                "{}.{}",
                                name, column.name
                            )),
                        );
                        TypeInfo {
                            data_type: sqlex_common::DataType::Custom("unknown".to_string()),
                            nullable: true,
                        }
                    },
                },
                None => {
                    self.diagnostics
                        .push(super::super::diagnostics::Diagnostic::unknown_table(name));
                    TypeInfo {
                        data_type: sqlex_common::DataType::Custom("unknown".to_string()),
                        nullable: true,
                    }
                },
            },
            BoundTableSource::Derived { query } => {
                self.lookup_derived_column(state, query, &column.name)
            },
            BoundTableSource::Cte { name } => {
                let cte = state.query.ctes.iter().rev().find(|cte| cte.name == *name);
                if let Some(cte) = cte {
                    let schema = self.output_schema_for_query(&cte.query);

                    let mut info = None;
                    if !cte.columns.is_empty() {
                        if let Some(index) = cte.columns.iter().position(|c| c == &column.name) {
                            if let Some(col) = schema.columns.get(index) {
                                info = Some(TypeInfo {
                                    data_type: col.data_type.clone(),
                                    nullable: col.nullability,
                                });
                            }
                        }
                    }

                    if let Some(info) = info {
                        info
                    } else if let Some(col) = schema.columns.iter().find(|c| c.name == column.name)
                    {
                        TypeInfo {
                            data_type: col.data_type.clone(),
                            nullable: col.nullability,
                        }
                    } else {
                        self.diagnostics.push(
                            super::super::diagnostics::Diagnostic::unknown_column(&column.name),
                        );
                        TypeInfo {
                            data_type: sqlex_common::DataType::Custom("unknown".to_string()),
                            nullable: true,
                        }
                    }
                } else {
                    let table = state.query.tables.get(column.table);
                    if table.columns.iter().any(|c| c == &column.name) {
                        TypeInfo {
                            data_type: sqlex_common::DataType::Custom("unknown".to_string()),
                            nullable: true,
                        }
                    } else {
                        self.diagnostics
                            .push(super::super::diagnostics::Diagnostic::unknown_cte(name));
                        TypeInfo {
                            data_type: sqlex_common::DataType::Custom("unknown".to_string()),
                            nullable: true,
                        }
                    }
                }
            },
        };

        if state.nullable_tables.contains(&column.table) {
            info.nullable = true;
        }

        info
    }

    fn lookup_derived_column(
        &mut self,
        _state: &QueryTypeState<'_>,
        query: &BoundQuery,
        column_name: &str,
    ) -> TypeInfo {
        let schema = self.output_schema_for_query(query);
        if let Some(col) = schema.columns.iter().find(|c| c.name == column_name) {
            TypeInfo {
                data_type: col.data_type.clone(),
                nullable: col.nullability,
            }
        } else {
            self.diagnostics
                .push(super::super::diagnostics::Diagnostic::unknown_column(
                    column_name,
                ));
            TypeInfo {
                data_type: sqlex_common::DataType::Custom("unknown".to_string()),
                nullable: true,
            }
        }
    }
}

fn infer_literal(value: &sqlparser::ast::Value) -> TypeInfo {
    match value {
        sqlparser::ast::Value::Number(num, _) => {
            let is_float = num.contains('.');
            TypeInfo {
                data_type: if is_float {
                    sqlex_common::DataType::Double
                } else {
                    sqlex_common::DataType::Int
                },
                nullable: false,
            }
        },
        sqlparser::ast::Value::Boolean(_) => TypeInfo {
            data_type: sqlex_common::DataType::Bool,
            nullable: false,
        },
        sqlparser::ast::Value::SingleQuotedString(_)
        | sqlparser::ast::Value::DoubleQuotedString(_) => TypeInfo {
            data_type: sqlex_common::DataType::Text,
            nullable: false,
        },
        sqlparser::ast::Value::Null => TypeInfo {
            data_type: sqlex_common::DataType::Custom("unknown".to_string()),
            nullable: true,
        },
        _ => TypeInfo {
            data_type: sqlex_common::DataType::Custom("unknown".to_string()),
            nullable: true,
        },
    }
}

pub(super) fn merge_types(
    existing: Option<sqlex_common::DataType>,
    next: sqlex_common::DataType,
) -> Option<sqlex_common::DataType> {
    match existing {
        None => {
            if matches!(next, sqlex_common::DataType::Custom(_)) {
                None
            } else {
                Some(next)
            }
        },
        Some(current) => {
            if matches!(next, sqlex_common::DataType::Custom(_)) || current == next {
                Some(current)
            } else {
                Some(promote_numeric(&current, &next))
            }
        },
    }
}

fn analyze_binary_type(
    left: &sqlex_common::DataType,
    op: &BinaryOperator,
    right: &sqlex_common::DataType,
) -> sqlex_common::DataType {
    use sqlex_common::DataType;

    match op {
        BinaryOperator::Plus
        | BinaryOperator::Minus
        | BinaryOperator::Multiply
        | BinaryOperator::Modulo => promote_numeric(left, right),
        BinaryOperator::Divide => DataType::Double,
        BinaryOperator::Gt
        | BinaryOperator::Lt
        | BinaryOperator::GtEq
        | BinaryOperator::LtEq
        | BinaryOperator::Eq
        | BinaryOperator::NotEq => DataType::Bool,
        BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Xor => DataType::Bool,
        BinaryOperator::StringConcat => DataType::Text,
        BinaryOperator::BitwiseOr
        | BinaryOperator::BitwiseAnd
        | BinaryOperator::BitwiseXor
        | BinaryOperator::PGBitwiseShiftLeft
        | BinaryOperator::PGBitwiseShiftRight => promote_numeric(left, right),
        _ => left.clone(),
    }
}

fn promote_numeric(
    a: &sqlex_common::DataType,
    b: &sqlex_common::DataType,
) -> sqlex_common::DataType {
    use sqlex_common::DataType::*;

    match (a, b) {
        (Double, _) | (_, Double) => Double,
        (Float, _) | (_, Float) => Float,
        (Decimal, _) | (_, Decimal) => Decimal,
        (BigInt, _) | (_, BigInt) => BigInt,
        (Int, _) | (_, Int) => Int,
        (SmallInt, _) | (_, SmallInt) => SmallInt,
        (TinyInt, TinyInt) => TinyInt,
        _ => a.clone(),
    }
}
