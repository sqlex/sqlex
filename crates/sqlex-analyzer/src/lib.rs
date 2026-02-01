//! SQL query analyzer for sqlex.
//!
//! This crate analyzes SQL SELECT statements and infers result set metadata
//! including column names, types, and nullability.

mod analyzer;
mod error;
mod resolver;
mod scope;
mod type_inference;

pub use analyzer::{AnalyzeResult, QueryAnalyzer};
pub use error::AnalyzeError;
