//! Geospatial models

use geo::Geometry as GeoGeometry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Coordinate Reference System identified by EPSG code
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crs {
    pub epsg: u32,
    pub name: String,
}

impl Crs {
    /// Create a new CRS with EPSG code and name
    pub fn new(epsg: u32, name: impl Into<String>) -> Self {
        Self {
            epsg,
            name: name.into(),
        }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistanceUnit {
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

/// Geometry validation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidityMode {
    /// Strict validation - reject any invalid geometries
    Strict,
    /// Lenient validation - attempt to fix invalid geometries
    Lenient,
}

/// A spatial feature with geometry, properties, and CRS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub id: FeatureId,
    pub geometry: Geometry,
    pub properties: HashMap<String, serde_json::Value>,
    pub crs: Crs,
}

/// Feature identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FeatureId(pub String);

impl From<String> for FeatureId {
    fn from(s: String) -> Self {
        FeatureId(s)
    }
}

impl From<&str> for FeatureId {
    fn from(s: &str) -> Self {
        FeatureId(s.to_string())
    }
}

/// Geometry wrapper around geo crate types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Geometry {
    Point(geo::Point),
    LineString(geo::LineString),
    Polygon(geo::Polygon),
    MultiPoint(geo::MultiPoint),
    MultiLineString(geo::MultiLineString),
    MultiPolygon(geo::MultiPolygon),
}

impl From<GeoGeometry> for Geometry {
    fn from(geom: GeoGeometry) -> Self {
        match geom {
            GeoGeometry::Point(p) => Geometry::Point(p),
            GeoGeometry::Line(l) => Geometry::LineString(vec![l.start, l.end].into()),
            GeoGeometry::LineString(ls) => Geometry::LineString(ls),
            GeoGeometry::Polygon(p) => Geometry::Polygon(p),
            GeoGeometry::MultiPoint(mp) => Geometry::MultiPoint(mp),
            GeoGeometry::MultiLineString(mls) => Geometry::MultiLineString(mls),
            GeoGeometry::MultiPolygon(mp) => Geometry::MultiPolygon(mp),
            GeoGeometry::GeometryCollection(gc) => {
                // For simplicity, take the first geometry or default to a point
                gc.into_iter()
                    .next()
                    .map(Geometry::from)
                    .unwrap_or_else(|| Geometry::Point(geo::Point::new(0.0, 0.0)))
            }
            GeoGeometry::Rect(r) => Geometry::Polygon(r.to_polygon()),
            GeoGeometry::Triangle(t) => Geometry::Polygon(t.to_polygon()),
        }
    }
}

impl From<Geometry> for GeoGeometry {
    fn from(geom: Geometry) -> Self {
        match geom {
            Geometry::Point(p) => GeoGeometry::Point(p),
            Geometry::LineString(ls) => GeoGeometry::LineString(ls),
            Geometry::Polygon(p) => GeoGeometry::Polygon(p),
            Geometry::MultiPoint(mp) => GeoGeometry::MultiPoint(mp),
            Geometry::MultiLineString(mls) => GeoGeometry::MultiLineString(mls),
            Geometry::MultiPolygon(mp) => GeoGeometry::MultiPolygon(mp),
        }
    }
}

/// Geometry type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeometryType {
    Point,
    LineString,
    Polygon,
    MultiPoint,
    MultiLineString,
    MultiPolygon,
    Mixed,
}

impl Geometry {
    /// Get the geometry type
    pub fn geometry_type(&self) -> GeometryType {
        match self {
            Geometry::Point(_) => GeometryType::Point,
            Geometry::LineString(_) => GeometryType::LineString,
            Geometry::Polygon(_) => GeometryType::Polygon,
            Geometry::MultiPoint(_) => GeometryType::MultiPoint,
            Geometry::MultiLineString(_) => GeometryType::MultiLineString,
            Geometry::MultiPolygon(_) => GeometryType::MultiPolygon,
        }
    }
}

/// Spatial predicate for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpatialPredicate {
    /// Feature is within the filter geometry
    Within,
    /// Feature intersects the filter geometry
    Intersects,
    /// Feature contains the filter geometry
    Contains,
    /// Feature is within the bounding box
    BoundingBox,
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

    /// Convert to meters
    pub fn to_meters(&self) -> f64 {
        self.unit.to_meters(self.value)
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

impl SpatialFilter {
    /// Create a new spatial filter
    pub fn new(predicate: SpatialPredicate, crs: Crs) -> Self {
        Self {
            predicate,
            geometry: None,
            distance: None,
            crs,
        }
    }

    /// Set the filter geometry
    pub fn with_geometry(mut self, geometry: Geometry) -> Self {
        self.geometry = Some(geometry);
        self
    }

    /// Set the distance for proximity queries
    pub fn with_distance(mut self, distance: Distance) -> Self {
        self.distance = Some(distance);
        self
    }
}
