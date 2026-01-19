//! Geo module for spatial operations
//!
//! This module provides spatial algorithms, CRS transforms, indexing, and validation.

pub mod index;
pub mod models;
pub mod spatial;
pub mod transform;
pub mod validation;

// Re-export key types for convenience
pub use index::{IndexedGeometry, SpatialIndex, SpatialIndexBuilder};
pub use models::{from_geo_geometry, to_geo_geometry, GeometryExt};
pub use spatial::{
    count_spatial_matches, evaluate_spatial_filter, filter_geometries, geodesic_distance,
};
pub use transform::{crs_match, normalize_geometries, normalize_geometry, reproject_geometry};
pub use validation::{fix_geometry, validate_geometry, ValidationError, ValidationResult};
