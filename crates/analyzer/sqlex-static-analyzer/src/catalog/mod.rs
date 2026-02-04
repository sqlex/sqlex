//! Catalog Component
//!
//! Manages database catalog information and handles all DDL statements.

mod catalog;
mod ddl;
mod types;

pub use catalog::Catalog;
pub use sqlex_common::Dialect;
pub use types::*;
