//! Canonical geometry types used across all georag crates.
//!
//! These types provide a bridge between GeoJSON serialization and the
//! computational geo crate types.

use serde::{Deserialize, Serialize};

/// Coordinate Reference System identified by EPSG code
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crs {
    pub epsg: u32,
    pub name: String,
}

impl Default for Crs {
    fn default() -> Self {
        Self::wgs84()
    }
}

impl Crs {
    pub fn new(epsg: u32, name: impl Into<String>) -> Self {
        Self { epsg, name: name.into() }
    }

    /// WGS 84 (EPSG:4326)
    pub fn wgs84() -> Self {
        Self::new(4326, "WGS 84")
    }

    /// Web Mercator (EPSG:3857)
    pub fn web_mercator() -> Self {
        Self::new(3857, "Web Mercator")
    }
}

/// Distance units for spatial operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DistanceUnit {
    #[default]
    Meters,
    Kilometers,
    Miles,
    Feet,
}

impl DistanceUnit {
    /// Convert a distance value to meters
    pub fn to_meters(&self, value: f64) -> f64 {
        match self {
            DistanceUnit::Meters => value,
            DistanceUnit::Kilometers => value * 1000.0,
            DistanceUnit::Miles => value * 1609.34,
            DistanceUnit::Feet => value * 0.3048,
        }
    }

    /// Convert a distance value from meters to this unit
    pub fn from_meters(&self, meters: f64) -> f64 {
        match self {
            DistanceUnit::Meters => meters,
            DistanceUnit::Kilometers => meters / 1000.0,
            DistanceUnit::Miles => meters / 1609.34,
            DistanceUnit::Feet => meters / 0.3048,
        }
    }
}

/// Distance with unit
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Distance {
    pub value: f64,
    pub unit: DistanceUnit,
}

impl Distance {
    /// Create a new distance
    pub fn new(value: f64, unit: DistanceUnit) -> Self {
        Self { value, unit }
    }

    /// Create distance in meters
    pub fn meters(value: f64) -> Self {
        Self::new(value, DistanceUnit::Meters)
    }

    /// Create distance in kilometers
    pub fn kilometers(value: f64) -> Self {
        Self::new(value, DistanceUnit::Kilometers)
    }

    /// Convert to meters
    pub fn to_meters(&self) -> f64 {
        self.unit.to_meters(self.value)
    }
}

/// Geometry validation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ValidityMode {
    /// Strict validation - reject any invalid geometries
    Strict,
    /// Lenient validation - attempt to fix invalid geometries
    #[default]
    Lenient,
}

/// Spatial predicate for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SpatialPredicate {
    /// Geometry is completely within the filter geometry
    Within,
    /// Geometry intersects (overlaps) the filter geometry
    #[default]
    Intersects,
    /// Geometry contains the filter geometry
    Contains,
    /// Bounding boxes intersect (fast approximation)
    BoundingBox,
    /// Geometry is within specified distance of filter geometry (geodesic)
    DWithin,
}

/// Geometry type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GeometryType {
    #[default]
    Point,
    LineString,
    Polygon,
    MultiPoint,
    MultiLineString,
    MultiPolygon,
    GeometryCollection,
    Mixed,
}

/// GeoJSON-compatible geometry representation
///
/// This enum directly maps to GeoJSON geometry types with coordinate arrays.
/// It can be serialized/deserialized as GeoJSON and converted to/from `geo` crate types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Geometry {
    Point {
        coordinates: [f64; 2],
    },
    LineString {
        coordinates: Vec<[f64; 2]>,
    },
    Polygon {
        coordinates: Vec<Vec<[f64; 2]>>,
    },
    MultiPoint {
        coordinates: Vec<[f64; 2]>,
    },
    MultiLineString {
        coordinates: Vec<Vec<[f64; 2]>>,
    },
    MultiPolygon {
        coordinates: Vec<Vec<Vec<[f64; 2]>>>,
    },
}

impl Geometry {
    /// Create a Point geometry
    pub fn point(x: f64, y: f64) -> Self {
        Geometry::Point { coordinates: [x, y] }
    }

    /// Create a LineString geometry
    pub fn line_string(coords: Vec<[f64; 2]>) -> Self {
        Geometry::LineString { coordinates: coords }
    }

    /// Create a Polygon geometry
    pub fn polygon(rings: Vec<Vec<[f64; 2]>>) -> Self {
        Geometry::Polygon { coordinates: rings }
    }

    /// Get the geometry type
    pub fn geometry_type(&self) -> GeometryType {
        match self {
            Geometry::Point { .. } => GeometryType::Point,
            Geometry::LineString { .. } => GeometryType::LineString,
            Geometry::Polygon { .. } => GeometryType::Polygon,
            Geometry::MultiPoint { .. } => GeometryType::MultiPoint,
            Geometry::MultiLineString { .. } => GeometryType::MultiLineString,
            Geometry::MultiPolygon { .. } => GeometryType::MultiPolygon,
        }
    }

    /// Try to parse from a serde_json::Value (GeoJSON)
    pub fn from_geojson(value: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(value.clone()).ok()
    }

    /// Convert to serde_json::Value (GeoJSON)
    pub fn to_geojson(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// Spatial filter for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialFilter {
    pub predicate: SpatialPredicate,
    pub geometry: Option<Geometry>,
    pub distance: Option<Distance>,
    pub crs: Crs,
}

impl Default for SpatialFilter {
    fn default() -> Self {
        Self {
            predicate: SpatialPredicate::Intersects,
            geometry: None,
            distance: None,
            crs: Crs::wgs84(),
        }
    }
}

impl SpatialFilter {
    /// Create a new spatial filter
    pub fn new(predicate: SpatialPredicate) -> Self {
        Self { predicate, ..Default::default() }
    }

    /// Create a new spatial filter with CRS
    pub fn with_crs(predicate: SpatialPredicate, crs: Crs) -> Self {
        Self { predicate, crs, ..Default::default() }
    }

    /// Set the filter geometry
    pub fn geometry(mut self, geometry: Geometry) -> Self {
        self.geometry = Some(geometry);
        self
    }

    /// Set the distance for proximity queries
    pub fn distance(mut self, distance: Distance) -> Self {
        self.distance = Some(distance);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geometry_serialization() {
        let point = Geometry::point(115.0, -8.5);
        let json = serde_json::to_string(&point).unwrap();
        assert!(json.contains("Point"));
        assert!(json.contains("115"));

        let parsed: Geometry = serde_json::from_str(&json).unwrap();
        assert_eq!(point, parsed);
    }

    #[test]
    fn test_polygon_serialization() {
        let polygon = Geometry::polygon(vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]]);
        let json = serde_json::to_string(&polygon).unwrap();
        assert!(json.contains("Polygon"));

        let parsed: Geometry = serde_json::from_str(&json).unwrap();
        assert_eq!(polygon, parsed);
    }

    #[test]
    fn test_spatial_filter_builder() {
        let filter = SpatialFilter::new(SpatialPredicate::DWithin)
            .geometry(Geometry::point(115.0, -8.5))
            .distance(Distance::meters(1000.0));

        assert_eq!(filter.predicate, SpatialPredicate::DWithin);
        assert!(filter.geometry.is_some());
        assert_eq!(filter.distance.unwrap().value, 1000.0);
    }

    #[test]
    fn test_distance_conversion() {
        let km = Distance::kilometers(5.0);
        assert!((km.to_meters() - 5000.0).abs() < 0.01);

        let m = Distance::meters(1609.34);
        assert!((DistanceUnit::Miles.from_meters(m.value) - 1.0).abs() < 0.01);
    }
}
