use sqlex_analyzer::error::AnalyzerError;
use sqlparser::ast::{Function, FunctionArguments};

use crate::{
    builder::RelationBuilder, functions::model::FunctionSignature, node::expression::Expression,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FunctionBindKind {
    Scalar,
    Aggregate,
    Window,
}

#[allow(dead_code)]
impl RelationBuilder<'_> {
    pub(super) fn build_function_expression(
        &mut self,
        function: &Function,
    ) -> Result<Expression, AnalyzerError> {
        let function_name = function.name.to_string().to_ascii_lowercase();
        let has_over = function.over.is_some();
        let arity = match &function.args {
            FunctionArguments::None => 0,
            FunctionArguments::Subquery(_) => 1,
            FunctionArguments::List(list) => list.args.len(),
        };

        let (bind_kind, signature) = self.resolve_function_bind_kind(&function_name, has_over)?;
        self.validate_function_arity(&function_name, arity, signature)?;

        let _ = (function, bind_kind);
        todo!("function expression builder is not implemented yet")
    }

    fn resolve_function_bind_kind(
        &self,
        function_name: &str,
        has_over: bool,
    ) -> Result<(FunctionBindKind, Option<&FunctionSignature>), AnalyzerError> {
        let scalar_signature = self.functions.resolve_scalar(function_name);
        let aggregate_signature = self.functions.resolve_aggregate(function_name);
        let window_signature = self.functions.resolve_window(function_name);

        if has_over {
            if let Some(signature) = window_signature.or(aggregate_signature) {
                return Ok((FunctionBindKind::Window, Some(signature)));
            }
            if scalar_signature.is_some() {
                return Err(AnalyzerError::analysis(
                    "A0008",
                    format!("function '{function_name}' does not support OVER clause"),
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
            return Err(AnalyzerError::analysis(
                "A0009",
                format!("window function '{function_name}' requires OVER clause"),
            ));
        }

        Ok((FunctionBindKind::Scalar, None))
    }

    fn validate_function_arity(
        &self,
        function_name: &str,
        arity: usize,
        signature: Option<&FunctionSignature>,
    ) -> Result<(), AnalyzerError> {
        let Some(signature) = signature else {
            return Ok(());
        };

        if arity < signature.min_arity {
            return Err(AnalyzerError::analysis(
                "A0007",
                format!(
                    "function '{function_name}' expects at least {} argument(s), got {arity}",
                    signature.min_arity
                ),
            ));
        }
        if let Some(max_arity) = signature.max_arity {
            if arity > max_arity {
                return Err(AnalyzerError::analysis(
                    "A0007",
                    format!(
                        "function '{function_name}' expects at most {} argument(s), got {arity}",
                        max_arity
                    ),
                ));
            }
        }

        Ok(())
    }
}
