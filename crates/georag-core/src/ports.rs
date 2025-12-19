//! Port trait definitions
//!
//! These traits define the interfaces that adapters must implement.

pub mod storage;

pub use storage::{DocumentStore, SpatialStore, VectorStore};
