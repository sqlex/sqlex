use sqlex_analyzer::extension::DataTypeExt;
use sqlex_common::types::DataType;
use sqlparser::ast::{self, BinaryOperator, UnaryOperator};

use crate::{
    analysis::{
        diagnostics::Diagnostic,
        infer::{Inferrer, QueryTypeState, TypeInfo},
    },
    ir::{
        bound::{BoundExpr, BoundTableSource},
        ids::{ColumnId, ExprId},
    },
};

impl Inferrer<'_> {
    pub(super) fn infer_expr(
        &mut self,
        state: &mut QueryTypeState<'_>,
        expr_id: ExprId,
    ) -> TypeInfo {
        if let Some(info) = state.types.get(&expr_id) {
            return info.clone();
        }

        let info = match state.stmt.exprs.get(expr_id) {
            BoundExpr::Column(column_id) => self.infer_column(state, *column_id),
            BoundExpr::Literal(value) => self.infer_literal(value),
            BoundExpr::Binary { left, op, right } => {
                self.infer_binary_expr(state, *left, op, *right)
            },
            BoundExpr::Unary { op, expr } => self.infer_unary_expr(state, *op, *expr),
            BoundExpr::IsNull { .. } => TypeInfo {
                data_type: self.boolean_result_type(),
                nullable: false,
            },
            BoundExpr::Function {
                name,
                kind,
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

                self.infer_function(name, kind, &arg_types, &arg_nullables, *distinct, *over)
            },
            BoundExpr::Case {
                operand,
                conditions,
                results,
                else_result,
            } => self.infer_case_expr(state, operand, conditions, results, else_result),
            BoundExpr::Subquery(subquery) => {
                let schema = self.output_schema_for_query_body(state, subquery);
                if schema.columns.len() == 1 {
                    let col = &schema.columns[0];
                    let guaranteed_row = self
                        .query_body_cardinality(state.stmt, subquery)
                        .guarantees_row();
                    TypeInfo {
                        data_type: col.data_type.clone(),
                        nullable: if guaranteed_row {
                            col.nullability
                        } else {
                            true
                        },
                    }
                } else {
                    self.diagnostics
                        .push(Diagnostic::scalar_subquery_column_count());
                    TypeInfo {
                        data_type: DataType::Custom("unknown".to_string()),
                        nullable: true,
                    }
                }
            },
            BoundExpr::InList { expr, .. } | BoundExpr::InSubquery { expr, .. } => {
                let expr_info = self.infer_expr(state, *expr);
                TypeInfo {
                    data_type: self.boolean_result_type(),
                    nullable: expr_info.nullable,
                }
            },
            BoundExpr::Error => TypeInfo {
                data_type: DataType::Custom("unknown".to_string()),
                nullable: true,
            },
        };

        state.types.insert(expr_id, info.clone());
        info
    }

    fn infer_binary_expr(
        &mut self,
        state: &mut QueryTypeState<'_>,
        left: ExprId,
        op: &BinaryOperator,
        right: ExprId,
    ) -> TypeInfo {
        let left_info = self.infer_expr(state, left);
        let right_info = self.infer_expr(state, right);
        if matches!(
            op,
            BinaryOperator::Plus
                | BinaryOperator::Minus
                | BinaryOperator::Multiply
                | BinaryOperator::Divide
                | BinaryOperator::Modulo
        ) && matches!(self.dialect, sqlex_common::dialect::Dialect::Postgres)
            && (!left_info.data_type.is_numeric() || !right_info.data_type.is_numeric())
        {
            self.diagnostics
                .push(Diagnostic::binary_operator_type_mismatch(
                    &format!("{op:?}"),
                    &format!("{:?}", left_info.data_type),
                    &format!("{:?}", right_info.data_type),
                ));
        }

        let data_type = match op {
            BinaryOperator::Plus
            | BinaryOperator::Minus
            | BinaryOperator::Multiply
            | BinaryOperator::Modulo => left_info.data_type.promote_numeric(&right_info.data_type),
            BinaryOperator::Divide => DataType::Double,
            BinaryOperator::Gt
            | BinaryOperator::Lt
            | BinaryOperator::GtEq
            | BinaryOperator::LtEq
            | BinaryOperator::Eq
            | BinaryOperator::NotEq
            | BinaryOperator::Spaceship
            | BinaryOperator::And
            | BinaryOperator::Or
            | BinaryOperator::Xor
            | BinaryOperator::PGOverlap
            | BinaryOperator::PGRegexMatch
            | BinaryOperator::PGRegexIMatch
            | BinaryOperator::PGRegexNotMatch
            | BinaryOperator::PGRegexNotIMatch
            | BinaryOperator::PGLikeMatch
            | BinaryOperator::PGILikeMatch
            | BinaryOperator::PGNotLikeMatch
            | BinaryOperator::PGNotILikeMatch
            | BinaryOperator::PGStartsWith
            | BinaryOperator::AtAt
            | BinaryOperator::AtArrow
            | BinaryOperator::ArrowAt
            | BinaryOperator::AtQuestion
            | BinaryOperator::Question
            | BinaryOperator::QuestionAnd
            | BinaryOperator::QuestionPipe
            | BinaryOperator::Overlaps => self.boolean_result_type(),
            BinaryOperator::StringConcat => DataType::Text,
            BinaryOperator::BitwiseOr
            | BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseXor
            | BinaryOperator::PGBitwiseShiftLeft
            | BinaryOperator::PGBitwiseShiftRight => {
                left_info.data_type.promote_numeric(&right_info.data_type)
            },
            _ => left_info.data_type.clone(),
        };
        TypeInfo {
            data_type,
            nullable: left_info.nullable || right_info.nullable,
        }
    }

    fn infer_unary_expr(
        &mut self,
        state: &mut QueryTypeState<'_>,
        op: UnaryOperator,
        expr: ExprId,
    ) -> TypeInfo {
        let inner = self.infer_expr(state, expr);
        match op {
            UnaryOperator::Not => TypeInfo {
                data_type: self.boolean_result_type(),
                nullable: inner.nullable,
            },
            UnaryOperator::Minus | UnaryOperator::Plus => inner,
            _ => inner,
        }
    }

    fn infer_case_expr(
        &mut self,
        state: &mut QueryTypeState<'_>,
        operand: &Option<ExprId>,
        conditions: &[ExprId],
        results: &[ExprId],
        else_result: &Option<ExprId>,
    ) -> TypeInfo {
        if let Some(expr_id) = operand {
            self.infer_expr(state, *expr_id);
        }
        for expr_id in conditions {
            self.infer_expr(state, *expr_id);
        }

        let mut merged_type: Option<DataType> = None;
        let mut nullable = else_result.is_none();

        for expr_id in results {
            let info = self.infer_expr(state, *expr_id);
            merged_type = DataType::merge_common_type(self.dialect, merged_type, &info.data_type);
            if info.nullable {
                nullable = true;
            }
        }

        if let Some(expr_id) = else_result {
            let info = self.infer_expr(state, *expr_id);
            merged_type = DataType::merge_common_type(self.dialect, merged_type, &info.data_type);
            if info.nullable {
                nullable = true;
            }
        }

        TypeInfo {
            data_type: merged_type.unwrap_or_else(|| DataType::Custom("unknown".to_string())),
            nullable,
        }
    }

    fn infer_column(&mut self, state: &mut QueryTypeState<'_>, column_id: ColumnId) -> TypeInfo {
        let column = state.stmt.columns.get(column_id);
        let table = state.stmt.tables.get(column.table);

        let mut info = match &table.source {
            BoundTableSource::Table { .. } => {
                if let (Some(dt), Some(null)) = (&column.data_type, column.nullable) {
                    TypeInfo {
                        data_type: dt.clone(),
                        nullable: null,
                    }
                } else {
                    TypeInfo {
                        data_type: DataType::Custom("unknown".to_string()),
                        nullable: true,
                    }
                }
            },
            BoundTableSource::Derived { query } => {
                self.lookup_derived_column(state, query, &column.name)
            },
            BoundTableSource::Cte { name } => self.lookup_cte_column(state, name, &column.name),
        };

        if state.nullable_tables.contains(&column.table) {
            info.nullable = true;
        }

        info
    }

    fn lookup_derived_column(
        &mut self,
        state: &mut QueryTypeState<'_>,
        query: &crate::ir::bound::BoundQueryBody,
        col_name: &str,
    ) -> TypeInfo {
        let schema = self.output_schema_for_query_body(state, query);
        schema
            .columns
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(col_name))
            .map(|c| TypeInfo {
                data_type: c.data_type.clone(),
                nullable: c.nullability,
            })
            .unwrap_or_else(|| TypeInfo {
                data_type: DataType::Custom("unknown".to_string()),
                nullable: true,
            })
    }

    fn lookup_cte_column(
        &mut self,
        state: &mut QueryTypeState<'_>,
        cte_name: &str,
        col_name: &str,
    ) -> TypeInfo {
        let cte = state
            .stmt
            .ctes
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(cte_name));
        let Some(cte) = cte else {
            return TypeInfo {
                data_type: DataType::Custom("unknown".to_string()),
                nullable: true,
            };
        };
        let cte_query = cte.query.clone();
        let schema = self.output_schema_for_query_body(state, &cte_query);
        schema
            .columns
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(col_name))
            .map(|c| TypeInfo {
                data_type: c.data_type.clone(),
                nullable: c.nullability,
            })
            .unwrap_or_else(|| TypeInfo {
                data_type: DataType::Custom("unknown".to_string()),
                nullable: true,
            })
    }

    fn infer_literal(&self, value: &ast::Value) -> TypeInfo {
        match value {
            ast::Value::Number(num, _) => {
                let has_exponent = num.contains('e') || num.contains('E');
                let has_decimal = num.contains('.');
                TypeInfo {
                    data_type: if has_exponent {
                        DataType::Double
                    } else if has_decimal {
                        match self.dialect {
                            sqlex_common::dialect::Dialect::MySQL
                            | sqlex_common::dialect::Dialect::Postgres => DataType::Decimal,
                            sqlex_common::dialect::Dialect::SQLite => DataType::Double,
                        }
                    } else {
                        match self.dialect {
                            sqlex_common::dialect::Dialect::MySQL => DataType::BigInt(false),
                            sqlex_common::dialect::Dialect::Postgres
                            | sqlex_common::dialect::Dialect::SQLite => DataType::Int(false),
                        }
                    },
                    nullable: false,
                }
            },
            ast::Value::Boolean(_) => TypeInfo {
                data_type: self.boolean_result_type(),
                nullable: false,
            },
            ast::Value::SingleQuotedString(_) | ast::Value::DoubleQuotedString(_) => TypeInfo {
                data_type: DataType::Text,
                nullable: false,
            },
            ast::Value::Null => TypeInfo {
                data_type: DataType::Custom("unknown".to_string()),
                nullable: true,
            },
            _ => TypeInfo {
                data_type: DataType::Custom("unknown".to_string()),
                nullable: true,
            },
        }
    }

    fn boolean_result_type(&self) -> DataType {
        match self.dialect {
            sqlex_common::dialect::Dialect::MySQL => DataType::BigInt(false),
            sqlex_common::dialect::Dialect::Postgres | sqlex_common::dialect::Dialect::SQLite => {
                DataType::Bool
            },
        }
    }
}
