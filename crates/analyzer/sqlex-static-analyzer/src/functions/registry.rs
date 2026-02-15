use std::collections::HashMap;

use sqlex_common::dialect::Dialect;

use crate::functions::{common, mysql, postgres, sqlite};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FunctionCategory {
    Scalar,
    Aggregate,
    Window,
}

#[derive(Debug, Clone)]
pub(crate) struct FunctionSignature {
    pub(crate) name: String,
    pub(crate) category: FunctionCategory,
    pub(crate) min_arity: usize,
    pub(crate) max_arity: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FunctionRegistry {
    signatures: HashMap<String, FunctionSignature>,
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

    pub(crate) fn register(
        &mut self,
        name: &str,
        category: FunctionCategory,
        min_arity: usize,
        max_arity: Option<usize>,
    ) {
        let key = name.to_ascii_lowercase();
        self.signatures.insert(
            key.clone(),
            FunctionSignature {
                name: key,
                category,
                min_arity,
                max_arity,
            },
        );
    }

    pub(crate) fn resolve(&self, name: &str) -> Option<&FunctionSignature> {
        self.signatures.get(&name.to_ascii_lowercase())
    }
}
