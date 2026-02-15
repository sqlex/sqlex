use sqlex_analyzer::extension::DataTypeExt;
use sqlex_common::{dialect::Dialect, types::DataType};
use sqlparser::ast::{
    CeilFloorKind, DateTimeField, Function, FunctionArg, FunctionArgExpr, FunctionArguments,
    WindowSpec, WindowType,
};

use crate::{
    algebra::{
        planner::{Algebraizer, context::BuildContext},
        scalar::{BoundLiteral, BoundScalarExpr, SortKey},
    },
    catalog::{
        model::Catalog,
        normalize::{normalize_ident, normalize_object_name},
    },
    diagnostics::{Diagnostic, Phase},
    functions::registry::{FunctionCategory, FunctionRegistry},
};

impl Algebraizer {
    pub(crate) fn bind_function(
        &self,
        function: &Function,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &BuildContext,
    ) -> Result<(BoundScalarExpr, bool), Diagnostic> {
        let function_name = normalize_object_name(&function.name, self.dialect);
        let function_name_lower = function_name.to_ascii_lowercase();

        let mut bound_args = Vec::new();
        let mut has_aggregate_in_args = false;

        match &function.args {
            FunctionArguments::None => {},
            FunctionArguments::Subquery(_) => {
                return Err(Diagnostic::todo(
                    Phase::Algebraize,
                    "function subquery argument binding",
                ));
            },
            FunctionArguments::List(argument_list) => {
                for arg in &argument_list.args {
                    let (bound_arg, arg_has_aggregate) =
                        self.bind_function_arg(arg, catalog, functions, context)?;
                    bound_args.push(bound_arg);
                    has_aggregate_in_args |= arg_has_aggregate;
                }
            },
        }

        self.validate_function_arity(&function_name_lower, bound_args.len(), functions)?;
        self.validate_function_argument_types(&function_name_lower, &bound_args, context)?;

        let distinct = matches!(
            &function.args,
            FunctionArguments::List(list)
                if list.duplicate_treatment.is_some_and(|value| matches!(value, sqlparser::ast::DuplicateTreatment::Distinct))
        );

        if function.over.is_some() {
            let (partition_by, order_by) = match &function.over {
                Some(WindowType::WindowSpec(spec)) => {
                    self.bind_window_spec(spec, catalog, functions, context)?
                },
                Some(WindowType::NamedWindow(window_name)) => {
                    let normalized_name = normalize_ident(window_name, self.dialect);
                    let Some(spec) = context.named_windows.get(&normalized_name) else {
                        return Err(Diagnostic::new(
                            "A3048",
                            Phase::Algebraize,
                            format!("unknown WINDOW definition: {normalized_name}"),
                        ));
                    };
                    self.bind_window_spec(spec, catalog, functions, context)?
                },
                None => (Vec::new(), Vec::new()),
            };

            return Ok((
                BoundScalarExpr::WindowCall {
                    name: function_name,
                    args: bound_args,
                    partition_by,
                    order_by,
                },
                has_aggregate_in_args,
            ));
        }

        let is_aggregate = functions
            .resolve(&function_name_lower)
            .is_some_and(|signature| signature.category == FunctionCategory::Aggregate);

        if is_aggregate {
            return Ok((
                BoundScalarExpr::AggregateCall {
                    name: function_name,
                    args: bound_args,
                    distinct,
                },
                true,
            ));
        }

        Ok((
            BoundScalarExpr::Function {
                name: function_name,
                args: bound_args,
            },
            has_aggregate_in_args,
        ))
    }

    fn bind_window_spec(
        &self,
        spec: &WindowSpec,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &BuildContext,
    ) -> Result<(Vec<BoundScalarExpr>, Vec<SortKey>), Diagnostic> {
        let resolved_spec = self.resolve_window_spec_for_over(spec, context)?;

        let mut partition_by = Vec::new();
        for expr in &resolved_spec.partition_by {
            let (bound_expr, _) = self.bind_expr(expr, catalog, functions, context)?;
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
            let (bound_expr, _) = self.bind_expr(&order_expr.expr, catalog, functions, context)?;
            order_by.push(SortKey {
                expr: bound_expr,
                asc: order_expr.asc.unwrap_or(true),
                nulls_first: order_expr.nulls_first,
            });
        }

        Ok((partition_by, order_by))
    }

    fn resolve_window_spec_for_over(
        &self,
        spec: &WindowSpec,
        context: &BuildContext,
    ) -> Result<WindowSpec, Diagnostic> {
        let mut resolved_spec = if let Some(base_name) = &spec.window_name {
            let normalized_base = normalize_ident(base_name, self.dialect);
            let Some(base_spec) = context.named_windows.get(&normalized_base) else {
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

    fn bind_function_arg(
        &self,
        arg: &FunctionArg,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &BuildContext,
    ) -> Result<(BoundScalarExpr, bool), Diagnostic> {
        let arg_expr = match arg {
            FunctionArg::Named { arg, .. } => arg,
            FunctionArg::ExprNamed { arg, .. } => arg,
            FunctionArg::Unnamed(arg) => arg,
        };

        match arg_expr {
            FunctionArgExpr::Expr(expr) => self.bind_expr(expr, catalog, functions, context),
            FunctionArgExpr::Wildcard => Ok((BoundScalarExpr::Placeholder("*".to_string()), false)),
            FunctionArgExpr::QualifiedWildcard(prefix) => Ok((
                BoundScalarExpr::Placeholder(format!(
                    "{}.*",
                    normalize_object_name(prefix, self.dialect)
                )),
                false,
            )),
        }
    }

    pub(crate) fn validate_function_arity(
        &self,
        function_name_lower: &str,
        arity: usize,
        functions: &FunctionRegistry,
    ) -> Result<(), Diagnostic> {
        let Some(signature) = functions.resolve(function_name_lower) else {
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
        bound_args: &[BoundScalarExpr],
        context: &BuildContext,
    ) -> Result<(), Diagnostic> {
        if matches!(self.dialect, Dialect::SQLite) {
            if matches!(function_name_lower, "ceil" | "floor") {
                self.require_postgres_numeric_arg(function_name_lower, bound_args, context, 0)?;
            }
            return Ok(());
        }

        if !matches!(self.dialect, Dialect::Postgres) {
            return Ok(());
        }

        match function_name_lower {
            "upper" | "lower" | "trim" | "ltrim" | "rtrim" | "length" | "char_length"
            | "substr" => {
                self.require_postgres_text_arg(function_name_lower, bound_args, context, 0)?;
            },
            "abs" | "ceil" | "floor" | "round" | "sqrt" | "exp" | "ln" | "log10" | "sign"
            | "sum" | "avg" => {
                self.require_postgres_numeric_arg(function_name_lower, bound_args, context, 0)?;
            },
            "power" | "mod" => {
                self.require_postgres_numeric_arg(function_name_lower, bound_args, context, 0)?;
                self.require_postgres_numeric_arg(function_name_lower, bound_args, context, 1)?;
            },
            _ => {},
        }

        Ok(())
    }

    pub(crate) fn bind_ceil_or_floor(
        &self,
        function_name: &str,
        expr: &sqlparser::ast::Expr,
        field: &CeilFloorKind,
        catalog: &Catalog,
        functions: &FunctionRegistry,
        context: &BuildContext,
    ) -> Result<(BoundScalarExpr, bool), Diagnostic> {
        let (bound_expr, has_aggregate) = self.bind_expr(expr, catalog, functions, context)?;
        let bound_args = match field {
            CeilFloorKind::DateTimeField(DateTimeField::NoDateTime) => vec![bound_expr],
            CeilFloorKind::DateTimeField(_) | CeilFloorKind::Scale(_) => {
                return Err(Diagnostic::todo(
                    Phase::Algebraize,
                    "CEIL/FLOOR modifiers binding",
                ));
            },
        };

        self.validate_function_arity(function_name, bound_args.len(), functions)?;
        self.validate_function_argument_types(function_name, &bound_args, context)?;

        Ok((
            BoundScalarExpr::Function {
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

    fn require_postgres_text_arg(
        &self,
        function_name_lower: &str,
        bound_args: &[BoundScalarExpr],
        context: &BuildContext,
        index: usize,
    ) -> Result<(), Diagnostic> {
        let Some(arg_type) = self.bound_expr_static_type(bound_args.get(index), context) else {
            return Ok(());
        };
        if arg_type.is_text_like() {
            return Ok(());
        }

        Err(Diagnostic::new(
            "A3022",
            Phase::Algebraize,
            format!(
                "function '{}' expects text argument at position {}",
                function_name_lower,
                index + 1
            ),
        ))
    }

    fn require_postgres_numeric_arg(
        &self,
        function_name_lower: &str,
        bound_args: &[BoundScalarExpr],
        context: &BuildContext,
        index: usize,
    ) -> Result<(), Diagnostic> {
        let Some(arg_type) = self.bound_expr_static_type(bound_args.get(index), context) else {
            return Ok(());
        };
        if arg_type.is_numeric() {
            return Ok(());
        }

        Err(Diagnostic::new(
            "A3023",
            Phase::Algebraize,
            format!(
                "function '{}' expects numeric argument at position {}",
                function_name_lower,
                index + 1
            ),
        ))
    }

    fn bound_expr_static_type(
        &self,
        expr: Option<&BoundScalarExpr>,
        context: &BuildContext,
    ) -> Option<DataType> {
        let expr = expr?;
        match expr {
            BoundScalarExpr::SlotRef(slot_id) => context
                .relation_scopes
                .iter()
                .flat_map(|scope| scope.schema.columns.iter())
                .find(|column| column.slot_id == *slot_id)
                .and_then(|column| column.data_type.clone()),
            BoundScalarExpr::Literal(literal) => self.bound_literal_static_type(literal),
            BoundScalarExpr::Cast { target_type, .. } => Some(target_type.clone()),
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
            BoundLiteral::Placeholder(_) => None,
        }
    }
}
