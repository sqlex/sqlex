//! E2E test framework for sqlex SQL analyzer.
//!
//! This crate provides infrastructure for testing sqlex's SQL analysis
//! against real databases using testcontainers.

pub mod config;
pub mod db;
pub mod error;
pub mod runner;

pub use config::{Dialect, TestCase, TestSuite};
pub use error::{Error, Result};
pub use runner::TestRunner;
