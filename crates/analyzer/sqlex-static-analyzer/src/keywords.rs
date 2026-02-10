use std::sync::OnceLock;

use sqlex_common::dialect::Dialect;
use sqlparser::ast::Ident;

static MYSQL_RESERVED: OnceLock<Vec<&'static str>> = OnceLock::new();
static POSTGRES_RESERVED: OnceLock<Vec<&'static str>> = OnceLock::new();

fn mysql_reserved() -> &'static [&'static str] {
    MYSQL_RESERVED
        .get_or_init(|| {
            let mut list: Vec<&'static str> = include_str!("keywords/mysql_reserved.txt")
                .lines()
                .collect();
            list.sort_unstable();
            list
        })
        .as_slice()
}

fn postgres_reserved() -> &'static [&'static str] {
    POSTGRES_RESERVED
        .get_or_init(|| {
            let mut list: Vec<&'static str> = include_str!("keywords/postgres_reserved.txt")
                .lines()
                .collect();
            list.sort_unstable();
            list
        })
        .as_slice()
}

pub(crate) fn is_reserved_identifier(dialect: Dialect, ident: &Ident) -> bool {
    if ident.quote_style.is_some() {
        return false;
    }

    let value = ident.value.to_ascii_uppercase();
    match dialect {
        Dialect::MySQL => mysql_reserved().binary_search(&value.as_str()).is_ok(),
        Dialect::Postgres => postgres_reserved().binary_search(&value.as_str()).is_ok(),
        Dialect::SQLite => false,
    }
}
