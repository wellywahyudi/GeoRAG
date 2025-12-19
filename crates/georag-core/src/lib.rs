//! GeoRAG Core - Domain models, workspace, and configuration
//!
//! This crate contains the core domain logic and port definitions for the GeoRAG system.

pub mod config;
pub mod error;
pub mod models;
pub mod ports;
pub mod processing;

pub use error::{GeoragError, Result};
