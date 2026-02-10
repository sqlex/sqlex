use sqlex_analyzer::extension::DataTypeExt;
use sqlex_common::types::DataType;

use super::{ColumnMetadata, Inferrer, TypeInfo};
use crate::{
    diagnostics::Diagnostic,
    functions,
    ir::scalar::{LiteralValue, ScalarExpr},
};

impl Inferrer<'_> {
    /// Infer the type and nullability of a scalar expression.
    /// `input_columns` provides the schema context from the input relation.
    pub(super) fn infer_scalar(
        &mut self,
        expr: &ScalarExpr,
        input_columns: &[ColumnMetadata],
    ) -> TypeInfo {
        match expr {
            ScalarExpr::ColumnRef { table, column } => {
                self.infer_column_ref(table.as_deref(), column, input_columns)
            },
            ScalarExpr::Literal(value) => self.infer_literal(value),
            ScalarExpr::BinaryOp { left, op, right } => {
                self.infer_binary_op(left, *op, right, input_columns)
            },
            ScalarExpr::UnaryOp { op, expr } => self.infer_unary_op(*op, expr, input_columns),
            ScalarExpr::Function { name, args } => {
                self.infer_function_call(name, args, input_columns, false)
            },
            ScalarExpr::Cast { expr, target_type } => {
                self.infer_cast(expr, target_type, input_columns)
            },
            ScalarExpr::AggregateCall {
                name,
                args,
                distinct,
            } => self.infer_aggregate_call(name, args, *distinct, input_columns),
            ScalarExpr::WindowCall {
                name,
                args,
                is_aggregate_window,
                ..
            } => self.infer_window_call(name, args, *is_aggregate_window, input_columns),
            ScalarExpr::IsNull { .. } => TypeInfo {
                data_type: self.boolean_result_type(),
                nullable: false,
            },
            ScalarExpr::InList { expr, .. } => {
                let expr_info = self.infer_scalar(expr, input_columns);
                TypeInfo {
                    data_type: self.boolean_result_type(),
                    nullable: expr_info.nullable,
                }
            },
            ScalarExpr::Between {
                expr, low, high, ..
            } => {
                let expr_info = self.infer_scalar(expr, input_columns);
                let low_info = self.infer_scalar(low, input_columns);
                let high_info = self.infer_scalar(high, input_columns);
                TypeInfo {
                    data_type: self.boolean_result_type(),
                    nullable: expr_info.nullable || low_info.nullable || high_info.nullable,
                }
            },
            ScalarExpr::Case {
                operand,
                when_clauses,
                else_result,
            } => self.infer_case(operand, when_clauses, else_result, input_columns),
            ScalarExpr::ScalarSubquery(subquery) => self.infer_scalar_subquery(subquery),
            ScalarExpr::InSubquery { expr, .. } => {
                let expr_info = self.infer_scalar(expr, input_columns);
                TypeInfo {
                    data_type: self.boolean_result_type(),
                    nullable: expr_info.nullable,
                }
            },
            ScalarExpr::Exists { .. } => TypeInfo {
                data_type: self.boolean_result_type(),
                nullable: false,
            },
            ScalarExpr::Wildcard => TypeInfo {
                data_type: DataType::Custom("*".to_string()),
                nullable: false,
            },
            ScalarExpr::QualifiedWildcard { .. } => TypeInfo {
                data_type: DataType::Custom("*".to_string()),
                nullable: false,
            },
            ScalarExpr::Error => TypeInfo {
                data_type: DataType::Custom("unknown".to_string()),
                nullable: true,
            },
        }
    }

    fn infer_column_ref(
        &mut self,
        table: Option<&str>,
        column: &str,
        input_columns: &[ColumnMetadata],
    ) -> TypeInfo {
        if let Some(table_name) = table {
            // Qualified column reference — match by table alias AND column name
            for col in input_columns {
                if let Some(ref col_table) = col.table {
                    if col_table.eq_ignore_ascii_case(table_name)
                        && col.name.eq_ignore_ascii_case(column)
                    {
                        return TypeInfo {
                            data_type: col.data_type.clone(),
                            nullable: col.nullable,
                        };
                    }
                }
            }
            // Fall back to catalog lookup (for aliases that map to real tables)
            if let Some(table_def) = self.catalog.get_table(table_name) {
                if let Some(col_def) = table_def.get_column(column) {
                    return TypeInfo {
                        data_type: col_def.data_type.clone(),
                        nullable: col_def.nullable,
                    };
                }
            }
        } else {
            // Unqualified column — check for ambiguity
            let matches: Vec<&ColumnMetadata> = input_columns
                .iter()
                .filter(|col| col.name.eq_ignore_ascii_case(column))
                .collect();

            if matches.len() > 1 {
                self.diagnostics.push(Diagnostic::ambiguous_column(column));
                return TypeInfo {
                    data_type: matches[0].data_type.clone(),
                    nullable: true,
                };
            }
            if let Some(col) = matches.first() {
                return TypeInfo {
                    data_type: col.data_type.clone(),
                    nullable: col.nullable,
                };
            }
        }

        // Column not found — report error
        let col_display = if let Some(t) = table {
            format!("{t}.{column}")
        } else {
            column.to_string()
        };
        self.diagnostics
            .push(Diagnostic::unknown_column(&col_display));
        TypeInfo {
            data_type: DataType::Custom("unknown".to_string()),
            nullable: true,
        }
    }

    fn infer_literal(&self, value: &LiteralValue) -> TypeInfo {
        match value {
            LiteralValue::Null => TypeInfo {
                data_type: DataType::Custom("unknown".to_string()),
                nullable: true,
            },
            LiteralValue::Boolean(_) => TypeInfo {
                data_type: self.boolean_result_type(),
                nullable: false,
            },
            LiteralValue::Integer(_) => TypeInfo {
                data_type: match self.dialect {
                    sqlex_common::dialect::Dialect::MySQL => DataType::BigInt,
                    sqlex_common::dialect::Dialect::Postgres
                    | sqlex_common::dialect::Dialect::SQLite => DataType::Int,
                },
                nullable: false,
            },
            LiteralValue::Float(_) => TypeInfo {
                data_type: match self.dialect {
                    sqlex_common::dialect::Dialect::MySQL
                    | sqlex_common::dialect::Dialect::Postgres => DataType::Decimal,
                    sqlex_common::dialect::Dialect::SQLite => DataType::Double,
                },
                nullable: false,
            },
            LiteralValue::String(_) => TypeInfo {
                data_type: match self.dialect {
                    sqlex_common::dialect::Dialect::MySQL => DataType::Varchar,
                    sqlex_common::dialect::Dialect::Postgres
                    | sqlex_common::dialect::Dialect::SQLite => DataType::Text,
                },
                nullable: false,
            },
        }
    }

    fn infer_binary_op(
        &mut self,
        left: &ScalarExpr,
        op: crate::ir::auxiliary::BinaryOp,
        right: &ScalarExpr,
        input_columns: &[ColumnMetadata],
    ) -> TypeInfo {
        use crate::ir::auxiliary::BinaryOp;

        let left_info = self.infer_scalar(left, input_columns);
        let right_info = self.infer_scalar(right, input_columns);

        if matches!(
            op,
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod
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
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Mod => {
                left_info.data_type.promote_numeric(&right_info.data_type)
            },
            BinaryOp::Div => DataType::Double,
            BinaryOp::Eq
            | BinaryOp::NotEq
            | BinaryOp::Lt
            | BinaryOp::LtEq
            | BinaryOp::Gt
            | BinaryOp::GtEq
            | BinaryOp::Spaceship
            | BinaryOp::And
            | BinaryOp::Or
            | BinaryOp::Xor
            | BinaryOp::Like
            | BinaryOp::NotLike
            | BinaryOp::PGRegexMatch
            | BinaryOp::PGRegexIMatch
            | BinaryOp::PGRegexNotMatch
            | BinaryOp::PGRegexNotIMatch
            | BinaryOp::PGLikeMatch
            | BinaryOp::PGILikeMatch
            | BinaryOp::PGNotLikeMatch
            | BinaryOp::PGNotILikeMatch
            | BinaryOp::PGStartsWith
            | BinaryOp::PGOverlap
            | BinaryOp::Overlaps
            | BinaryOp::AtAt
            | BinaryOp::AtArrow
            | BinaryOp::ArrowAt
            | BinaryOp::AtQuestion
            | BinaryOp::Question
            | BinaryOp::QuestionAnd
            | BinaryOp::QuestionPipe => self.boolean_result_type(),
            BinaryOp::StringConcat => DataType::Text,
            BinaryOp::BitwiseOr
            | BinaryOp::BitwiseAnd
            | BinaryOp::BitwiseXor
            | BinaryOp::BitwiseShiftLeft
            | BinaryOp::BitwiseShiftRight => {
                left_info.data_type.promote_numeric(&right_info.data_type)
            },
        };

        TypeInfo {
            data_type,
            nullable: left_info.nullable || right_info.nullable,
        }
    }

    fn infer_unary_op(
        &mut self,
        op: crate::ir::auxiliary::UnaryOp,
        expr: &ScalarExpr,
        input_columns: &[ColumnMetadata],
    ) -> TypeInfo {
        use crate::ir::auxiliary::UnaryOp;

        let inner = self.infer_scalar(expr, input_columns);
        match op {
            UnaryOp::Not => TypeInfo {
                data_type: self.boolean_result_type(),
                nullable: inner.nullable,
            },
            UnaryOp::Neg | UnaryOp::Plus => inner,
        }
    }

    fn infer_function_call(
        &mut self,
        name: &str,
        args: &[ScalarExpr],
        input_columns: &[ColumnMetadata],
        over: bool,
    ) -> TypeInfo {
        let function = functions::resolve_function(name);
        let upper = name.to_uppercase();

        let arg_types: Vec<DataType> = args
            .iter()
            .map(|a| self.infer_scalar(a, input_columns).data_type)
            .collect();
        let arg_nullables: Vec<bool> = args
            .iter()
            .map(|a| self.infer_scalar(a, input_columns).nullable)
            .collect();

        if function.arity().matches(arg_types.len()) {
            if let Some(detail) = function.validate_argument_types(self.dialect, &arg_types) {
                self.diagnostics
                    .push(Diagnostic::function_argument_type_mismatch(&upper, detail));
            }
        }

        let (data_type, nullable) =
            function.infer_type(self.dialect, &arg_types, &arg_nullables, over);
        TypeInfo {
            data_type,
            nullable,
        }
    }

    fn infer_cast(
        &mut self,
        expr: &ScalarExpr,
        target_type: &DataType,
        input_columns: &[ColumnMetadata],
    ) -> TypeInfo {
        let inner = self.infer_scalar(expr, input_columns);
        TypeInfo {
            data_type: target_type.clone(),
            nullable: inner.nullable,
        }
    }

    fn infer_aggregate_call(
        &mut self,
        name: &str,
        args: &[ScalarExpr],
        _distinct: bool,
        input_columns: &[ColumnMetadata],
    ) -> TypeInfo {
        let function = functions::resolve_function(name);
        let upper = name.to_uppercase();

        let arg_types: Vec<DataType> = args
            .iter()
            .map(|a| self.infer_scalar(a, input_columns).data_type)
            .collect();
        let arg_nullables: Vec<bool> = args
            .iter()
            .map(|a| self.infer_scalar(a, input_columns).nullable)
            .collect();

        if function.arity().matches(arg_types.len()) {
            if let Some(detail) = function.validate_argument_types(self.dialect, &arg_types) {
                self.diagnostics
                    .push(Diagnostic::function_argument_type_mismatch(&upper, detail));
            }
        }

        let (data_type, nullable) =
            function.infer_type(self.dialect, &arg_types, &arg_nullables, false);
        TypeInfo {
            data_type,
            nullable,
        }
    }

    fn infer_window_call(
        &mut self,
        name: &str,
        args: &[ScalarExpr],
        is_aggregate_window: bool,
        input_columns: &[ColumnMetadata],
    ) -> TypeInfo {
        let function = functions::resolve_function(name);
        let upper = name.to_uppercase();

        let arg_types: Vec<DataType> = args
            .iter()
            .map(|a| self.infer_scalar(a, input_columns).data_type)
            .collect();
        let arg_nullables: Vec<bool> = args
            .iter()
            .map(|a| self.infer_scalar(a, input_columns).nullable)
            .collect();

        if function.arity().matches(arg_types.len()) {
            if let Some(detail) = function.validate_argument_types(self.dialect, &arg_types) {
                self.diagnostics
                    .push(Diagnostic::function_argument_type_mismatch(&upper, detail));
            }
        }

        // Window functions always use over=true for type inference
        let (data_type, nullable) =
            function.infer_type(self.dialect, &arg_types, &arg_nullables, true);
        let _ = is_aggregate_window;
        TypeInfo {
            data_type,
            nullable,
        }
    }

    fn infer_case(
        &mut self,
        operand: &Option<Box<ScalarExpr>>,
        when_clauses: &[crate::ir::scalar::WhenClause],
        else_result: &Option<Box<ScalarExpr>>,
        input_columns: &[ColumnMetadata],
    ) -> TypeInfo {
        if let Some(op) = operand {
            self.infer_scalar(op, input_columns);
        }
        for clause in when_clauses {
            self.infer_scalar(&clause.condition, input_columns);
        }

        let mut merged_type: Option<DataType> = None;
        let mut nullable = else_result.is_none();

        for clause in when_clauses {
            let info = self.infer_scalar(&clause.result, input_columns);
            merged_type = DataType::merge_common_type(self.dialect, merged_type, &info.data_type);
            if info.nullable {
                nullable = true;
            }
        }

        if let Some(el) = else_result {
            let info = self.infer_scalar(el, input_columns);
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

    fn infer_scalar_subquery(
        &mut self,
        subquery: &crate::ir::relational::RelationalExpr,
    ) -> TypeInfo {
        let schema = self.infer_expr(subquery);
        if schema.columns.len() == 1 {
            let col = &schema.columns[0];
            let guaranteed_row = schema.cardinality.guarantees_row();
            TypeInfo {
                data_type: col.data_type.clone(),
                nullable: if guaranteed_row { col.nullable } else { true },
            }
        } else {
            self.diagnostics
                .push(Diagnostic::scalar_subquery_column_count());
            TypeInfo {
                data_type: DataType::Custom("unknown".to_string()),
                nullable: true,
            }
        }
    }

    pub(super) fn boolean_result_type(&self) -> DataType {
        match self.dialect {
            sqlex_common::dialect::Dialect::MySQL => DataType::BigInt,
            sqlex_common::dialect::Dialect::Postgres | sqlex_common::dialect::Dialect::SQLite => {
                DataType::Bool
            },
        }
    }
}
