//! CRS transformation and normalization

use crate::models::{Crs, Geometry};
use georag_core::error::{GeoragError, Result};
use proj::Proj;

/// Check if two CRS are the same
pub fn crs_match(crs1: &Crs, crs2: &Crs) -> bool {
    crs1.epsg == crs2.epsg
}

/// Detect CRS mismatch and return error if they don't match
pub fn check_crs_mismatch(dataset_crs: &Crs, workspace_crs: &Crs) -> Result<()> {
    if !crs_match(dataset_crs, workspace_crs) {
        return Err(GeoragError::CrsMismatch {
            dataset_crs: format!("EPSG:{} ({})", dataset_crs.epsg, dataset_crs.name),
            workspace_crs: format!("EPSG:{} ({})", workspace_crs.epsg, workspace_crs.name),
        });
    }
    Ok(())
}

/// Transform a coordinate pair using a projection
fn transform_coord(proj: &Proj, x: f64, y: f64) -> Result<(f64, f64)> {
    proj.convert((x, y)).map_err(|e| GeoragError::ConfigInvalid {
        key: "crs".to_string(),
        reason: format!("Projection failed: {}", e),
    })
}

/// Reproject a geometry from one CRS to another
pub fn reproject_geometry(geometry: &Geometry, from_crs: &Crs, to_crs: &Crs) -> Result<Geometry> {
    // If CRS are the same, no transformation needed
    if crs_match(from_crs, to_crs) {
        return Ok(geometry.clone());
    }

    // Create projection
    let from_proj = format!("EPSG:{}", from_crs.epsg);
    let to_proj = format!("EPSG:{}", to_crs.epsg);

    let proj = Proj::new_known_crs(&from_proj, &to_proj, None).map_err(|e| {
        GeoragError::ConfigInvalid {
            key: "crs".to_string(),
            reason: format!("Failed to create projection from {} to {}: {}", from_proj, to_proj, e),
        }
    })?;

    // Transform the geometry based on type
    let transformed = match geometry {
        Geometry::Point { coordinates } => {
            let (x, y) = transform_coord(&proj, coordinates[0], coordinates[1])?;
            Geometry::Point { coordinates: [x, y] }
        }
        Geometry::LineString { coordinates } => {
            let coords: Result<Vec<[f64; 2]>> = coordinates
                .iter()
                .map(|c| transform_coord(&proj, c[0], c[1]).map(|(x, y)| [x, y]))
                .collect();
            Geometry::LineString { coordinates: coords? }
        }
        Geometry::Polygon { coordinates } => {
            let rings: Result<Vec<Vec<[f64; 2]>>> = coordinates
                .iter()
                .map(|ring| {
                    ring.iter()
                        .map(|c| transform_coord(&proj, c[0], c[1]).map(|(x, y)| [x, y]))
                        .collect()
                })
                .collect();
            Geometry::Polygon { coordinates: rings? }
        }
        Geometry::MultiPoint { coordinates } => {
            let coords: Result<Vec<[f64; 2]>> = coordinates
                .iter()
                .map(|c| transform_coord(&proj, c[0], c[1]).map(|(x, y)| [x, y]))
                .collect();
            Geometry::MultiPoint { coordinates: coords? }
        }
        Geometry::MultiLineString { coordinates } => {
            let lines: Result<Vec<Vec<[f64; 2]>>> = coordinates
                .iter()
                .map(|line| {
                    line.iter()
                        .map(|c| transform_coord(&proj, c[0], c[1]).map(|(x, y)| [x, y]))
                        .collect()
                })
                .collect();
            Geometry::MultiLineString { coordinates: lines? }
        }
        Geometry::MultiPolygon { coordinates } => {
            let polygons: Result<Vec<Vec<Vec<[f64; 2]>>>> = coordinates
                .iter()
                .map(|poly| {
                    poly.iter()
                        .map(|ring| {
                            ring.iter()
                                .map(|c| transform_coord(&proj, c[0], c[1]).map(|(x, y)| [x, y]))
                                .collect()
                        })
                        .collect()
                })
                .collect();
            Geometry::MultiPolygon { coordinates: polygons? }
        }
    };

    Ok(transformed)
}

/// Alias for [`reproject_geometry`] with domain-specific naming.
pub fn normalize_geometry(geometry: &Geometry, from_crs: &Crs, target_crs: &Crs) -> Result<Geometry> {
    reproject_geometry(geometry, from_crs, target_crs)
}

/// Normalize a collection of geometries to a target CRS
pub fn normalize_geometries(
    geometries: &[(Geometry, Crs)],
    target_crs: &Crs,
) -> Result<Vec<Geometry>> {
    geometries.iter().map(|(geom, crs)| normalize_geometry(geom, crs, target_crs)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crs_match() {
        let wgs84 = Crs::wgs84();
        let wgs84_2 = Crs::new(4326, "WGS 84 Copy");
        let mercator = Crs::web_mercator();

        assert!(crs_match(&wgs84, &wgs84_2));
        assert!(!crs_match(&wgs84, &mercator));
    }

    #[test]
    fn test_same_crs_no_transform() {
        let geom = Geometry::point(115.0, -8.5);
        let wgs84 = Crs::wgs84();

        let result = reproject_geometry(&geom, &wgs84, &wgs84).unwrap();
        assert_eq!(geom, result);
    }
}
