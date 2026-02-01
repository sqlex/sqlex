//! SQL query analyzer for sqlex.
//!
//! This crate analyzes SQL SELECT statements and infers result set metadata
//! including column names, types, and nullability.

mod error;
mod resolver;
mod scope;
mod type_inference;
mod analyzer;

pub use error::AnalyzeError;
pub use analyzer::{QueryAnalyzer, AnalyzeResult};
