//! Planner Component
//!
//! Converts DML statements into query plan trees with type and nullability inference.

mod builder;
mod nullability;
mod plan;
mod types;

// Re-export BuildContext for external use
pub use builder::BuildContext;
pub use plan::*;
