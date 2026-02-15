use std::collections::HashMap;

use sqlex_common::dialect::Dialect;

use crate::functions::{common, mysql, postgres, sqlite};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FunctionCategory {
    Scalar,
    Aggregate,
    Window,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FunctionArgType {
    TextLike,
    Numeric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FunctionArgTypeRule {
    pub(crate) index: usize,
    pub(crate) expected: FunctionArgType,
}

impl FunctionArgTypeRule {
    pub(crate) const fn text(index: usize) -> Self {
        Self {
            index,
            expected: FunctionArgType::TextLike,
        }
    }

    pub(crate) const fn numeric(index: usize) -> Self {
        Self {
            index,
            expected: FunctionArgType::Numeric,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FunctionCoercionProfile {
    Strict,
    Permissive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FunctionReturnTypeRule {
    TextLikeOrDefaultText,
    LengthInteger,
    NumericUnary,
    NumericBinaryCommon,
    CoalesceCommonType,
    NullIfFirstArg,
    Count,
    Sum,
    Avg,
    MinMax,
    Ranking,
    LeadLag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FunctionNullabilityRule {
    AnyArg,
    AllArgs,
    Always,
    Never,
}

#[derive(Debug, Clone)]
pub(crate) struct FunctionSignature {
    pub(crate) category: FunctionCategory,
    pub(crate) min_arity: usize,
    pub(crate) max_arity: Option<usize>,
    pub(crate) return_type_rule: FunctionReturnTypeRule,
    pub(crate) nullability_rule: FunctionNullabilityRule,
    pub(crate) coercion_profile: FunctionCoercionProfile,
    pub(crate) arg_type_rules: Vec<FunctionArgTypeRule>,
}

impl FunctionSignature {
    pub(crate) fn new(
        category: FunctionCategory,
        min_arity: usize,
        max_arity: Option<usize>,
        return_type_rule: FunctionReturnTypeRule,
        nullability_rule: FunctionNullabilityRule,
    ) -> Self {
        Self {
            category,
            min_arity,
            max_arity,
            return_type_rule,
            nullability_rule,
            coercion_profile: FunctionCoercionProfile::Permissive,
            arg_type_rules: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct FunctionOverloads {
    scalar: Option<FunctionSignature>,
    aggregate: Option<FunctionSignature>,
    window: Option<FunctionSignature>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FunctionRegistry {
    signatures: HashMap<String, FunctionOverloads>,
}

impl FunctionRegistry {
    pub(crate) fn new(dialect: Dialect) -> Self {
        let mut registry = Self::default();
        common::register_common_functions(&mut registry);

        match dialect {
            Dialect::Postgres => postgres::register_postgres_functions(&mut registry),
            Dialect::MySQL => mysql::register_mysql_functions(&mut registry),
            Dialect::SQLite => sqlite::register_sqlite_functions(&mut registry),
        }

        registry
    }

    pub(crate) fn register(&mut self, name: &str, signature: FunctionSignature) {
        let key = name.to_ascii_lowercase();
        let entry = self.signatures.entry(key).or_default();
        match signature.category {
            FunctionCategory::Scalar => entry.scalar = Some(signature),
            FunctionCategory::Aggregate => entry.aggregate = Some(signature),
            FunctionCategory::Window => entry.window = Some(signature),
        }
    }

    pub(crate) fn set_arg_type_rules(
        &mut self,
        name: &str,
        category: FunctionCategory,
        coercion_profile: FunctionCoercionProfile,
        arg_type_rules: &[FunctionArgTypeRule],
    ) {
        let Some(signature) = self.resolve_mut(name, category) else {
            return;
        };
        signature.coercion_profile = coercion_profile;
        signature.arg_type_rules = arg_type_rules.to_vec();
    }

    pub(crate) fn resolve_scalar(&self, name: &str) -> Option<&FunctionSignature> {
        self.signatures
            .get(&name.to_ascii_lowercase())
            .and_then(|overloads| overloads.scalar.as_ref())
    }

    pub(crate) fn resolve_aggregate(&self, name: &str) -> Option<&FunctionSignature> {
        self.signatures
            .get(&name.to_ascii_lowercase())
            .and_then(|overloads| overloads.aggregate.as_ref())
    }

    pub(crate) fn resolve_window(&self, name: &str) -> Option<&FunctionSignature> {
        self.signatures
            .get(&name.to_ascii_lowercase())
            .and_then(|overloads| overloads.window.as_ref())
    }

    pub(crate) fn resolve_window_call(&self, name: &str) -> Option<&FunctionSignature> {
        self.resolve_window(name)
            .or_else(|| self.resolve_aggregate(name))
    }

    fn resolve_mut(
        &mut self,
        name: &str,
        category: FunctionCategory,
    ) -> Option<&mut FunctionSignature> {
        let key = name.to_ascii_lowercase();
        let overloads = self.signatures.get_mut(&key)?;
        match category {
            FunctionCategory::Scalar => overloads.scalar.as_mut(),
            FunctionCategory::Aggregate => overloads.aggregate.as_mut(),
            FunctionCategory::Window => overloads.window.as_mut(),
        }
    }
}
