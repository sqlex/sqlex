use sqlex_analyzer::extension::DataTypeExt;
use sqlex_common::{dialect::Dialect, types::DataType};
use sqlparser::ast::{
    CeilFloorKind, DateTimeField, Function, FunctionArg, FunctionArgExpr, FunctionArguments,
    WindowSpec, WindowType,
};

use crate::{
    algebraizer::{
        Algebraizer,
        model::{
            expression::{BoundLiteral, Expression},
            schema::SortKey,
        },
    },
    catalog::normalize::{normalize_ident, normalize_object_name},
    diagnostics::{Diagnostic, Phase},
    functions::model::{FunctionArgType, FunctionCoercionProfile, FunctionSignature},
};

enum FunctionBindKind {
    Scalar,
    Aggregate,
    Window,
}

impl Algebraizer<'_> {
    pub(crate) fn build_function_expression(
        &mut self,
        function: &Function,
    ) -> Result<(Expression, bool), Diagnostic> {
        let function_name = normalize_object_name(&function.name, self.dialect);
        let function_name_lower = function_name.to_ascii_lowercase();

        let mut bound_args = Vec::new();
        let mut has_aggregate_in_args = false;

        match &function.args {
            FunctionArguments::None => {},
            FunctionArguments::Subquery(query) => {
                let bound_subquery = self
                    .build_single_column_subquery_relation(query, "function subquery argument")?;
                bound_args.push(Expression::ScalarSubquery(Box::new(bound_subquery)));
            },
            FunctionArguments::List(argument_list) => {
                for arg in &argument_list.args {
                    let (bound_arg, arg_has_aggregate) = self.build_function_arg_expression(arg)?;
                    bound_args.push(bound_arg);
                    has_aggregate_in_args |= arg_has_aggregate;
                }
            },
        }

        let (bind_kind, signature) =
            self.resolve_function_call_signature(&function_name_lower, function.over.is_some())?;
        self.validate_function_arity(&function_name_lower, bound_args.len(), signature)?;
        self.validate_function_argument_types(&function_name_lower, &bound_args, signature)?;

        let distinct = matches!(
            &function.args,
            FunctionArguments::List(list)
                if list.duplicate_treatment.is_some_and(|value| matches!(value, sqlparser::ast::DuplicateTreatment::Distinct))
        );

        if matches!(bind_kind, FunctionBindKind::Window) {
            let (partition_by, order_by) = match &function.over {
                Some(WindowType::WindowSpec(spec)) => self.build_window_spec_expression(spec)?,
                Some(WindowType::NamedWindow(window_name)) => {
                    let normalized_name = normalize_ident(window_name, self.dialect);
                    let Some(spec) = self.current_named_windows().get(&normalized_name).cloned()
                    else {
                        return Err(Diagnostic::new(
                            "A3048",
                            Phase::Algebraize,
                            format!("unknown WINDOW definition: {normalized_name}"),
                        ));
                    };
                    self.build_window_spec_expression(&spec)?
                },
                None => {
                    return Err(Diagnostic::new(
                        "A3059",
                        Phase::Algebraize,
                        format!("window function '{function_name_lower}' requires OVER clause"),
                    ));
                },
            };

            return Ok((
                Expression::WindowCall {
                    name: function_name,
                    args: bound_args,
                    partition_by,
                    order_by,
                },
                has_aggregate_in_args,
            ));
        }

        if matches!(bind_kind, FunctionBindKind::Aggregate) {
            return Ok((
                Expression::AggregateCall {
                    name: function_name,
                    args: bound_args,
                    distinct,
                },
                true,
            ));
        }

        Ok((
            Expression::Function {
                name: function_name,
                args: bound_args,
            },
            has_aggregate_in_args,
        ))
    }

    fn resolve_function_call_signature(
        &self,
        function_name_lower: &str,
        has_over: bool,
    ) -> Result<(FunctionBindKind, Option<&FunctionSignature>), Diagnostic> {
        let scalar_signature = self.functions.resolve_scalar(function_name_lower);
        let aggregate_signature = self.functions.resolve_aggregate(function_name_lower);
        let window_signature = self.functions.resolve_window(function_name_lower);

        if has_over {
            if let Some(signature) = window_signature.or(aggregate_signature) {
                return Ok((FunctionBindKind::Window, Some(signature)));
            }
            if scalar_signature.is_some() {
                return Err(Diagnostic::new(
                    "A3058",
                    Phase::Algebraize,
                    format!("function '{function_name_lower}' does not support OVER clause"),
                ));
            }
            return Ok((FunctionBindKind::Window, None));
        }

        if let Some(signature) = aggregate_signature {
            return Ok((FunctionBindKind::Aggregate, Some(signature)));
        }
        if let Some(signature) = scalar_signature {
            return Ok((FunctionBindKind::Scalar, Some(signature)));
        }
        if window_signature.is_some() {
            return Err(Diagnostic::new(
                "A3059",
                Phase::Algebraize,
                format!("window function '{function_name_lower}' requires OVER clause"),
            ));
        }

        Ok((FunctionBindKind::Scalar, None))
    }

    fn build_window_spec_expression(
        &mut self,
        spec: &WindowSpec,
    ) -> Result<(Vec<Expression>, Vec<SortKey>), Diagnostic> {
        let resolved_spec = self.resolve_window_spec_for_over(spec)?;

        let mut partition_by = Vec::new();
        for expr in &resolved_spec.partition_by {
            let (bound_expr, _) = self.build_expression(expr)?;
            partition_by.push(bound_expr);
        }

        let mut order_by = Vec::new();
        for order_expr in &resolved_spec.order_by {
            if order_expr.with_fill.is_some() {
                return Err(Diagnostic::new(
                    "A3031",
                    Phase::Algebraize,
                    "ORDER BY WITH FILL is not supported in this iteration",
                ));
            }
            let (bound_expr, _) = self.build_expression(&order_expr.expr)?;
            order_by.push(SortKey {
                expr: bound_expr,
                asc: order_expr.asc.unwrap_or(true),
                nulls_first: order_expr.nulls_first,
            });
        }

        Ok((partition_by, order_by))
    }

    fn resolve_window_spec_for_over(&self, spec: &WindowSpec) -> Result<WindowSpec, Diagnostic> {
        let mut resolved_spec = if let Some(base_name) = &spec.window_name {
            let normalized_base = normalize_ident(base_name, self.dialect);
            let Some(base_spec) = self.current_named_windows().get(&normalized_base) else {
                return Err(Diagnostic::new(
                    "A3048",
                    Phase::Algebraize,
                    format!("unknown WINDOW definition: {normalized_base}"),
                ));
            };
            base_spec.clone()
        } else {
            WindowSpec {
                window_name: None,
                partition_by: Vec::new(),
                order_by: Vec::new(),
                window_frame: None,
            }
        };

        if !spec.partition_by.is_empty() {
            resolved_spec.partition_by = spec.partition_by.clone();
        }
        if !spec.order_by.is_empty() {
            resolved_spec.order_by = spec.order_by.clone();
        }
        if spec.window_frame.is_some() {
            resolved_spec.window_frame = spec.window_frame.clone();
        }
        resolved_spec.window_name = None;

        Ok(resolved_spec)
    }

    fn build_function_arg_expression(
        &mut self,
        arg: &FunctionArg,
    ) -> Result<(Expression, bool), Diagnostic> {
        let arg_expr = match arg {
            FunctionArg::Named { arg, .. } => arg,
            FunctionArg::ExprNamed { arg, .. } => arg,
            FunctionArg::Unnamed(arg) => arg,
        };

        match arg_expr {
            FunctionArgExpr::Expr(expr) => self.build_expression(expr),
            FunctionArgExpr::Wildcard => Ok((Expression::Placeholder, false)),
            FunctionArgExpr::QualifiedWildcard(_) => Ok((Expression::Placeholder, false)),
        }
    }

    pub(crate) fn validate_function_arity(
        &self,
        function_name_lower: &str,
        arity: usize,
        signature: Option<&FunctionSignature>,
    ) -> Result<(), Diagnostic> {
        let Some(signature) = signature else {
            return Ok(());
        };

        if arity < signature.min_arity {
            return Err(Diagnostic::new(
                "A3020",
                Phase::Algebraize,
                format!(
                    "function '{}' expects at least {} argument(s), got {}",
                    function_name_lower, signature.min_arity, arity
                ),
            ));
        }
        if let Some(max_arity) = signature.max_arity {
            if arity > max_arity {
                return Err(Diagnostic::new(
                    "A3021",
                    Phase::Algebraize,
                    format!(
                        "function '{}' expects at most {} argument(s), got {}",
                        function_name_lower, max_arity, arity
                    ),
                ));
            }
        }

        Ok(())
    }

    pub(crate) fn validate_function_argument_types(
        &self,
        function_name_lower: &str,
        bound_args: &[Expression],
        signature: Option<&FunctionSignature>,
    ) -> Result<(), Diagnostic> {
        let Some(signature) = signature else {
            return Ok(());
        };
        if matches!(
            signature.coercion_profile,
            FunctionCoercionProfile::Permissive
        ) {
            return Ok(());
        }

        for rule in &signature.arg_type_rules {
            let Some(arg_type) = self.bound_expr_static_type(bound_args.get(rule.index)) else {
                continue;
            };
            let matches_rule = match rule.expected {
                FunctionArgType::TextLike => arg_type.is_text_like(),
                FunctionArgType::Numeric => arg_type.is_numeric(),
            };
            if matches_rule {
                continue;
            }

            let (code, requirement_label) = match rule.expected {
                FunctionArgType::TextLike => ("A3022", "text"),
                FunctionArgType::Numeric => ("A3023", "numeric"),
            };
            return Err(Diagnostic::new(
                code,
                Phase::Algebraize,
                format!(
                    "function '{}' expects {} argument at position {}",
                    function_name_lower,
                    requirement_label,
                    rule.index + 1
                ),
            ));
        }

        Ok(())
    }

    pub(crate) fn build_ceil_or_floor_expression(
        &mut self,
        function_name: &str,
        expr: &sqlparser::ast::Expr,
        field: &CeilFloorKind,
    ) -> Result<(Expression, bool), Diagnostic> {
        let (bound_expr, has_aggregate) = self.build_expression(expr)?;
        let bound_args = match field {
            CeilFloorKind::DateTimeField(DateTimeField::NoDateTime) => vec![bound_expr],
            CeilFloorKind::DateTimeField(_) | CeilFloorKind::Scale(_) => {
                return Err(Diagnostic::new(
                    "A3060",
                    Phase::Algebraize,
                    "CEIL/FLOOR modifiers are not supported in this iteration",
                ));
            },
        };

        let signature = self.functions.resolve_scalar(function_name);
        self.validate_function_arity(function_name, bound_args.len(), signature)?;
        self.validate_function_argument_types(function_name, &bound_args, signature)?;

        Ok((
            Expression::Function {
                name: function_name.to_string(),
                args: bound_args,
            },
            has_aggregate,
        ))
    }

    pub(crate) fn validate_alias_ident(
        &self,
        alias: &sqlparser::ast::Ident,
    ) -> Result<(), Diagnostic> {
        if !matches!(self.dialect, Dialect::MySQL) {
            return Ok(());
        }
        if alias.quote_style.is_some() {
            return Ok(());
        }

        let alias_lower = alias.value.to_ascii_lowercase();
        if matches!(
            alias_lower.as_str(),
            "select" | "window" | "rank" | "row_number"
        ) {
            return Err(Diagnostic::new(
                "A3024",
                Phase::Algebraize,
                format!("reserved keyword cannot be used as alias: {}", alias.value),
            ));
        }

        Ok(())
    }

    fn bound_expr_static_type(&self, expr: Option<&Expression>) -> Option<DataType> {
        let expr = expr?;
        match expr {
            Expression::SlotRef(slot_id) => self
                .current_relation_bindings()
                .iter()
                .flat_map(|scope| scope.schema.columns.iter())
                .find(|column| column.slot_id == *slot_id)
                .and_then(|column| column.data_type.clone()),
            Expression::Literal(literal) => self.bound_literal_static_type(literal),
            Expression::Cast { target_type, .. } => Some(target_type.clone()),
            _ => None,
        }
    }

    fn bound_literal_static_type(&self, literal: &BoundLiteral) -> Option<DataType> {
        match literal {
            BoundLiteral::Null => None,
            BoundLiteral::Bool(_) => Some(match self.dialect {
                Dialect::Postgres => DataType::Bool,
                Dialect::MySQL | Dialect::SQLite => DataType::BigInt,
            }),
            BoundLiteral::Int { .. } => Some(match self.dialect {
                Dialect::Postgres => DataType::Int,
                Dialect::MySQL | Dialect::SQLite => DataType::BigInt,
            }),
            BoundLiteral::Float(_) => Some(match self.dialect {
                Dialect::SQLite => DataType::Double,
                Dialect::MySQL | Dialect::Postgres => DataType::Decimal,
            }),
            BoundLiteral::String(_) => Some(match self.dialect {
                Dialect::MySQL => DataType::Varchar,
                Dialect::Postgres | Dialect::SQLite => DataType::Text,
            }),
            BoundLiteral::Placeholder => None,
        }
    }
}
