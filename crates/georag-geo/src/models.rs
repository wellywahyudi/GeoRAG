//! Geometry models for georag-geo.
//!
//! This module re-exports canonical types from `georag-core` and provides
//! additional geo-specific functionality like conversions to/from the `geo` crate.

use geo::Geometry as GeoGeometry;

// Re-export canonical types from georag-core
pub use georag_core::models::{
    Crs, Distance, DistanceUnit, Geometry, GeometryType, SpatialFilter, SpatialPredicate,
    ValidityMode,
};

/// Convert a canonical Geometry to a geo::Geometry
pub fn to_geo_geometry(geom: &Geometry) -> GeoGeometry {
    match geom {
        Geometry::Point { coordinates } => {
            GeoGeometry::Point(geo::Point::new(coordinates[0], coordinates[1]))
        }
        Geometry::LineString { coordinates } => {
            let coords: Vec<geo::Coord> =
                coordinates.iter().map(|c| geo::Coord { x: c[0], y: c[1] }).collect();
            GeoGeometry::LineString(geo::LineString::new(coords))
        }
        Geometry::Polygon { coordinates } => {
            let rings: Vec<geo::LineString> = coordinates
                .iter()
                .map(|ring| {
                    let coords: Vec<geo::Coord> =
                        ring.iter().map(|c| geo::Coord { x: c[0], y: c[1] }).collect();
                    geo::LineString::new(coords)
                })
                .collect();
            if rings.is_empty() {
                GeoGeometry::Polygon(geo::Polygon::new(geo::LineString::new(vec![]), vec![]))
            } else {
                let exterior = rings[0].clone();
                let interiors: Vec<geo::LineString> = rings.into_iter().skip(1).collect();
                GeoGeometry::Polygon(geo::Polygon::new(exterior, interiors))
            }
        }
        Geometry::MultiPoint { coordinates } => {
            let points: Vec<geo::Point> =
                coordinates.iter().map(|c| geo::Point::new(c[0], c[1])).collect();
            GeoGeometry::MultiPoint(geo::MultiPoint::new(points))
        }
        Geometry::MultiLineString { coordinates } => {
            let lines: Vec<geo::LineString> = coordinates
                .iter()
                .map(|line| {
                    let coords: Vec<geo::Coord> =
                        line.iter().map(|c| geo::Coord { x: c[0], y: c[1] }).collect();
                    geo::LineString::new(coords)
                })
                .collect();
            GeoGeometry::MultiLineString(geo::MultiLineString::new(lines))
        }
        Geometry::MultiPolygon { coordinates } => {
            let polygons: Vec<geo::Polygon> = coordinates
                .iter()
                .map(|poly| {
                    let rings: Vec<geo::LineString> = poly
                        .iter()
                        .map(|ring| {
                            let coords: Vec<geo::Coord> =
                                ring.iter().map(|c| geo::Coord { x: c[0], y: c[1] }).collect();
                            geo::LineString::new(coords)
                        })
                        .collect();
                    if rings.is_empty() {
                        geo::Polygon::new(geo::LineString::new(vec![]), vec![])
                    } else {
                        let exterior = rings[0].clone();
                        let interiors: Vec<geo::LineString> = rings.into_iter().skip(1).collect();
                        geo::Polygon::new(exterior, interiors)
                    }
                })
                .collect();
            GeoGeometry::MultiPolygon(geo::MultiPolygon::new(polygons))
        }
    }
}

/// Convert a geo::Geometry to a canonical Geometry
pub fn from_geo_geometry(geom: &GeoGeometry) -> Geometry {
    match geom {
        GeoGeometry::Point(p) => Geometry::Point { coordinates: [p.x(), p.y()] },
        GeoGeometry::Line(l) => Geometry::LineString {
            coordinates: vec![[l.start.x, l.start.y], [l.end.x, l.end.y]],
        },
        GeoGeometry::LineString(ls) => Geometry::LineString {
            coordinates: ls.coords().map(|c| [c.x, c.y]).collect(),
        },
        GeoGeometry::Polygon(p) => {
            let mut rings = Vec::new();
            let exterior: Vec<[f64; 2]> = p.exterior().coords().map(|c| [c.x, c.y]).collect();
            rings.push(exterior);
            for interior in p.interiors() {
                let ring: Vec<[f64; 2]> = interior.coords().map(|c| [c.x, c.y]).collect();
                rings.push(ring);
            }
            Geometry::Polygon { coordinates: rings }
        }
        GeoGeometry::MultiPoint(mp) => Geometry::MultiPoint {
            coordinates: mp.iter().map(|p| [p.x(), p.y()]).collect(),
        },
        GeoGeometry::MultiLineString(mls) => Geometry::MultiLineString {
            coordinates: mls.iter().map(|ls| ls.coords().map(|c| [c.x, c.y]).collect()).collect(),
        },
        GeoGeometry::MultiPolygon(mp) => Geometry::MultiPolygon {
            coordinates: mp
                .iter()
                .map(|p| {
                    let mut rings = Vec::new();
                    let exterior: Vec<[f64; 2]> =
                        p.exterior().coords().map(|c| [c.x, c.y]).collect();
                    rings.push(exterior);
                    for interior in p.interiors() {
                        let ring: Vec<[f64; 2]> = interior.coords().map(|c| [c.x, c.y]).collect();
                        rings.push(ring);
                    }
                    rings
                })
                .collect(),
        },
        GeoGeometry::GeometryCollection(gc) => {
            // Take the first geometry or return an empty point
            gc.iter()
                .next()
                .map(from_geo_geometry)
                .unwrap_or_else(|| Geometry::Point { coordinates: [0.0, 0.0] })
        }
        GeoGeometry::Rect(r) => from_geo_geometry(&GeoGeometry::Polygon(r.to_polygon())),
        GeoGeometry::Triangle(t) => from_geo_geometry(&GeoGeometry::Polygon(t.to_polygon())),
    }
}

/// Extension trait for Geometry with geo-crate operations
pub trait GeometryExt {
    /// Convert to geo::Geometry
    fn to_geo(&self) -> GeoGeometry;

    /// Get the centroid as coordinates
    fn centroid_coords(&self) -> Option<[f64; 2]>;
}

impl GeometryExt for Geometry {
    fn to_geo(&self) -> GeoGeometry {
        to_geo_geometry(self)
    }

    fn centroid_coords(&self) -> Option<[f64; 2]> {
        use geo::algorithm::centroid::Centroid;
        let geo_geom = self.to_geo();
        geo_geom.centroid().map(|p| [p.x(), p.y()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_roundtrip() {
        let geom = Geometry::point(115.0, -8.5);
        let geo_geom = to_geo_geometry(&geom);
        let back = from_geo_geometry(&geo_geom);

        if let (Geometry::Point { coordinates: orig }, Geometry::Point { coordinates: converted }) =
            (&geom, &back)
        {
            assert!((orig[0] - converted[0]).abs() < 1e-10);
            assert!((orig[1] - converted[1]).abs() < 1e-10);
        } else {
            panic!("Expected Point geometry");
        }
    }

    #[test]
    fn test_polygon_roundtrip() {
        let geom = Geometry::polygon(vec![vec![
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
            [0.0, 0.0],
        ]]);
        let geo_geom = to_geo_geometry(&geom);
        let back = from_geo_geometry(&geo_geom);

        assert!(matches!(back, Geometry::Polygon { .. }));
    }

    #[test]
    fn test_centroid() {
        let geom = Geometry::polygon(vec![vec![
            [0.0, 0.0],
            [2.0, 0.0],
            [2.0, 2.0],
            [0.0, 2.0],
            [0.0, 0.0],
        ]]);
        let centroid = geom.centroid_coords().unwrap();
        assert!((centroid[0] - 1.0).abs() < 1e-10);
        assert!((centroid[1] - 1.0).abs() < 1e-10);
    }
}
