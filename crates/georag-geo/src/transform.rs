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

    // Transform the geometry
    let transformed = match geometry {
        Geometry::Point(p) => {
            let (x, y) = proj.convert((p.x(), p.y())).map_err(|e| GeoragError::ConfigInvalid {
                key: "crs".to_string(),
                reason: format!("Projection failed: {}", e),
            })?;
            Geometry::Point(geo::Point::new(x, y))
        }
        Geometry::LineString(ls) => {
            let coords: Result<Vec<_>> =
                ls.0.iter()
                    .map(|coord| {
                        proj.convert((coord.x, coord.y)).map(|(x, y)| geo::Coord { x, y }).map_err(
                            |e| GeoragError::ConfigInvalid {
                                key: "crs".to_string(),
                                reason: format!("Projection failed: {}", e),
                            },
                        )
                    })
                    .collect();
            Geometry::LineString(geo::LineString::from(coords?))
        }
        Geometry::Polygon(poly) => {
            let exterior_coords: Result<Vec<_>> = poly
                .exterior()
                .0
                .iter()
                .map(|coord| {
                    proj.convert((coord.x, coord.y)).map(|(x, y)| geo::Coord { x, y }).map_err(
                        |e| GeoragError::ConfigInvalid {
                            key: "crs".to_string(),
                            reason: format!("Projection failed: {}", e),
                        },
                    )
                })
                .collect();

            let interiors: Result<Vec<_>> = poly
                .interiors()
                .iter()
                .map(|interior| {
                    let coords: Result<Vec<_>> = interior
                        .0
                        .iter()
                        .map(|coord| {
                            proj.convert((coord.x, coord.y))
                                .map(|(x, y)| geo::Coord { x, y })
                                .map_err(|e| GeoragError::ConfigInvalid {
                                    key: "crs".to_string(),
                                    reason: format!("Projection failed: {}", e),
                                })
                        })
                        .collect();
                    coords.map(geo::LineString::from)
                })
                .collect();

            Geometry::Polygon(geo::Polygon::new(
                geo::LineString::from(exterior_coords?),
                interiors?,
            ))
        }
        Geometry::MultiPoint(mp) => {
            let points: Result<Vec<_>> =
                mp.0.iter()
                    .map(|p| {
                        proj.convert((p.x(), p.y())).map(|(x, y)| geo::Point::new(x, y)).map_err(
                            |e| GeoragError::ConfigInvalid {
                                key: "crs".to_string(),
                                reason: format!("Projection failed: {}", e),
                            },
                        )
                    })
                    .collect();
            Geometry::MultiPoint(geo::MultiPoint(points?))
        }
        Geometry::MultiLineString(mls) => {
            let linestrings: Result<Vec<_>> = mls
                .0
                .iter()
                .map(|ls| {
                    let coords: Result<Vec<_>> =
                        ls.0.iter()
                            .map(|coord| {
                                proj.convert((coord.x, coord.y))
                                    .map(|(x, y)| geo::Coord { x, y })
                                    .map_err(|e| GeoragError::ConfigInvalid {
                                        key: "crs".to_string(),
                                        reason: format!("Projection failed: {}", e),
                                    })
                            })
                            .collect();
                    coords.map(geo::LineString::from)
                })
                .collect();
            Geometry::MultiLineString(geo::MultiLineString(linestrings?))
        }
        Geometry::MultiPolygon(mp) => {
            let polygons: Result<Vec<_>> =
                mp.0.iter()
                    .map(|poly| {
                        let exterior_coords: Result<Vec<_>> = poly
                            .exterior()
                            .0
                            .iter()
                            .map(|coord| {
                                proj.convert((coord.x, coord.y))
                                    .map(|(x, y)| geo::Coord { x, y })
                                    .map_err(|e| GeoragError::ConfigInvalid {
                                        key: "crs".to_string(),
                                        reason: format!("Projection failed: {}", e),
                                    })
                            })
                            .collect();

                        let interiors: Result<Vec<_>> = poly
                            .interiors()
                            .iter()
                            .map(|interior| {
                                let coords: Result<Vec<_>> = interior
                                    .0
                                    .iter()
                                    .map(|coord| {
                                        proj.convert((coord.x, coord.y))
                                            .map(|(x, y)| geo::Coord { x, y })
                                            .map_err(|e| GeoragError::ConfigInvalid {
                                                key: "crs".to_string(),
                                                reason: format!("Projection failed: {}", e),
                                            })
                                    })
                                    .collect();
                                coords.map(geo::LineString::from)
                            })
                            .collect();

                        Ok(geo::Polygon::new(geo::LineString::from(exterior_coords?), interiors?))
                    })
                    .collect();
            Geometry::MultiPolygon(geo::MultiPolygon(polygons?))
        }
    };

    Ok(transformed)
}

/// Alias for [`reproject_geometry`] with domain-specific naming.
pub fn normalize_geometry(
    geometry: &Geometry,
    from_crs: &Crs,
    target_crs: &Crs,
) -> Result<Geometry> {
    reproject_geometry(geometry, from_crs, target_crs)
}

/// Normalize a collection of geometries to a target CRS
pub fn normalize_geometries(
    geometries: &[(Geometry, Crs)],
    target_crs: &Crs,
) -> Result<Vec<Geometry>> {
    geometries
        .iter()
        .map(|(geom, crs)| normalize_geometry(geom, crs, target_crs))
        .collect()
}
