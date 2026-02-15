use sqlex_common::dialect::Dialect;
use sqlparser::ast::{Ident, ObjectName};

pub(crate) fn normalize_ident(ident: &Ident, dialect: Dialect) -> String {
    match dialect {
        Dialect::Postgres => {
            if ident.quote_style.is_none() {
                ident.value.to_lowercase()
            } else {
                ident.value.clone()
            }
        },
        Dialect::MySQL | Dialect::SQLite => ident.value.clone(),
    }
}

pub(crate) fn normalize_object_name(name: &ObjectName, dialect: Dialect) -> String {
    name.0
        .iter()
        .map(|ident| normalize_ident(ident, dialect))
        .collect::<Vec<_>>()
        .join(".")
}

pub(crate) fn original_object_name(name: &ObjectName) -> String {
    name.0
        .iter()
        .map(|ident| ident.value.clone())
        .collect::<Vec<_>>()
        .join(".")
}
