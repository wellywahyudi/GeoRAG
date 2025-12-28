use crate::models::{Geometry, ValidityMode};
use georag_core::error::{GeoragError, Result};

/// Validation result with details
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
}

/// Validation error with location details
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub location: String,
    pub reason: String,
}

impl ValidationResult {
    /// Create a valid result
    pub fn valid() -> Self {
        Self { is_valid: true, errors: Vec::new() }
    }

    /// Create an invalid result with errors
    pub fn invalid(errors: Vec<ValidationError>) -> Self {
        Self { is_valid: false, errors }
    }

    /// Add an error to the result
    pub fn add_error(&mut self, location: String, reason: String) {
        self.is_valid = false;
        self.errors.push(ValidationError { location, reason });
    }
}

/// Validate a geometry
pub fn validate_geometry(geometry: &Geometry, _mode: ValidityMode) -> ValidationResult {
    match geometry {
        Geometry::Point(p) => validate_point(p),
        Geometry::LineString(ls) => validate_linestring(ls),
        Geometry::Polygon(poly) => validate_polygon(poly),
        Geometry::MultiPoint(mp) => validate_multipoint(mp),
        Geometry::MultiLineString(mls) => validate_multilinestring(mls),
        Geometry::MultiPolygon(mp) => validate_multipolygon(mp),
    }
}

fn validate_point(point: &geo::Point) -> ValidationResult {
    // Check for NaN or infinite coordinates
    if !point.x().is_finite() || !point.y().is_finite() {
        let mut result = ValidationResult::valid();
        result.add_error(
            format!("Point({}, {})", point.x(), point.y()),
            "Coordinates must be finite".to_string(),
        );
        return result;
    }
    ValidationResult::valid()
}

fn validate_linestring(linestring: &geo::LineString) -> ValidationResult {
    let mut result = ValidationResult::valid();

    // LineString must have at least 2 points
    if linestring.0.len() < 2 {
        result.add_error(
            "LineString".to_string(),
            format!("LineString must have at least 2 points, found {}", linestring.0.len()),
        );
        return result;
    }

    // Check each point
    for (i, coord) in linestring.0.iter().enumerate() {
        if !coord.x.is_finite() || !coord.y.is_finite() {
            result
                .add_error(format!("LineString[{}]", i), "Coordinates must be finite".to_string());
        }
    }

    result
}

fn validate_polygon(polygon: &geo::Polygon) -> ValidationResult {
    let mut result = ValidationResult::valid();

    // Check exterior ring
    let exterior = polygon.exterior();
    if exterior.0.len() < 4 {
        result.add_error(
            "Polygon exterior".to_string(),
            format!("Polygon exterior must have at least 4 points, found {}", exterior.0.len()),
        );
    }

    // Check if closed
    if let (Some(first), Some(last)) = (exterior.0.first(), exterior.0.last()) {
        if first != last {
            result.add_error(
                "Polygon exterior".to_string(),
                "Polygon exterior must be closed (first point == last point)".to_string(),
            );
        }
    }

    // Check interior rings
    for (i, interior) in polygon.interiors().iter().enumerate() {
        if interior.0.len() < 4 {
            result.add_error(
                format!("Polygon interior[{}]", i),
                format!("Polygon interior must have at least 4 points, found {}", interior.0.len()),
            );
        }

        // Check if closed
        if let (Some(first), Some(last)) = (interior.0.first(), interior.0.last()) {
            if first != last {
                result.add_error(
                    format!("Polygon interior[{}]", i),
                    "Polygon interior must be closed (first point == last point)".to_string(),
                );
            }
        }
    }

    result
}

fn validate_multipoint(multipoint: &geo::MultiPoint) -> ValidationResult {
    let mut result = ValidationResult::valid();
    for (i, point) in multipoint.0.iter().enumerate() {
        if !point.x().is_finite() || !point.y().is_finite() {
            result
                .add_error(format!("MultiPoint[{}]", i), "Coordinates must be finite".to_string());
        }
    }

    result
}

fn validate_multilinestring(multilinestring: &geo::MultiLineString) -> ValidationResult {
    let mut result = ValidationResult::valid();

    for (i, linestring) in multilinestring.0.iter().enumerate() {
        let ls_result = validate_linestring(linestring);
        if !ls_result.is_valid {
            for error in ls_result.errors {
                result
                    .add_error(format!("MultiLineString[{}].{}", i, error.location), error.reason);
            }
        }
    }

    result
}

fn validate_multipolygon(multipolygon: &geo::MultiPolygon) -> ValidationResult {
    let mut result = ValidationResult::valid();

    for (i, polygon) in multipolygon.0.iter().enumerate() {
        let poly_result = validate_polygon(polygon);
        if !poly_result.is_valid {
            for error in poly_result.errors {
                result.add_error(format!("MultiPolygon[{}].{}", i, error.location), error.reason);
            }
        }
    }

    result
}

/// Attempt to fix invalid geometries
pub fn fix_geometry(geometry: &Geometry) -> Result<Geometry> {
    match geometry {
        Geometry::Polygon(poly) => {
            let validation = validate_polygon(poly);
            if validation.is_valid {
                Ok(geometry.clone())
            } else {
                Err(GeoragError::InvalidGeometry {
                    feature_id: "unknown".to_string(),
                    reason: validation
                        .errors
                        .first()
                        .map(|e| e.reason.clone())
                        .unwrap_or_else(|| "Invalid geometry".to_string()),
                })
            }
        }
        _ => {
            let validation = validate_geometry(geometry, ValidityMode::Strict);
            if validation.is_valid {
                Ok(geometry.clone())
            } else {
                Err(GeoragError::InvalidGeometry {
                    feature_id: "unknown".to_string(),
                    reason: validation
                        .errors
                        .first()
                        .map(|e| e.reason.clone())
                        .unwrap_or_else(|| "Invalid geometry".to_string()),
                })
            }
        }
    }
}

/// Count invalid geometries in a collection
pub fn count_invalid_geometries(geometries: &[Geometry], mode: ValidityMode) -> usize {
    geometries.iter().filter(|g| !validate_geometry(g, mode).is_valid).count()
}
