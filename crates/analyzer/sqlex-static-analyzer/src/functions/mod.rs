use std::collections::HashMap;

use sqlex_common::dialect::Dialect;

use crate::functions::model::{
    FunctionArgTypeRule, FunctionCategory, FunctionCoercionProfile, FunctionSignature,
};

pub(crate) mod common;
pub(crate) mod model;
pub(crate) mod mysql;
pub(crate) mod postgres;
pub(crate) mod sqlite;

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
