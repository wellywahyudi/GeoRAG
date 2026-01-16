//! Geometry validation utilities

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

/// Check if a coordinate is valid (finite)
fn is_valid_coord(coord: &[f64; 2]) -> bool {
    coord[0].is_finite() && coord[1].is_finite()
}

/// Validate a geometry
pub fn validate_geometry(geometry: &Geometry, _mode: ValidityMode) -> ValidationResult {
    match geometry {
        Geometry::Point { coordinates } => validate_point(coordinates),
        Geometry::LineString { coordinates } => validate_linestring(coordinates),
        Geometry::Polygon { coordinates } => validate_polygon(coordinates),
        Geometry::MultiPoint { coordinates } => validate_multipoint(coordinates),
        Geometry::MultiLineString { coordinates } => validate_multilinestring(coordinates),
        Geometry::MultiPolygon { coordinates } => validate_multipolygon(coordinates),
    }
}

fn validate_point(coords: &[f64; 2]) -> ValidationResult {
    if !is_valid_coord(coords) {
        let mut result = ValidationResult::valid();
        result.add_error(
            format!("Point({}, {})", coords[0], coords[1]),
            "Coordinates must be finite".to_string(),
        );
        return result;
    }
    ValidationResult::valid()
}

fn validate_linestring(coords: &[[f64; 2]]) -> ValidationResult {
    let mut result = ValidationResult::valid();

    if coords.len() < 2 {
        result.add_error(
            "LineString".to_string(),
            format!("LineString must have at least 2 points, found {}", coords.len()),
        );
        return result;
    }

    for (i, coord) in coords.iter().enumerate() {
        if !is_valid_coord(coord) {
            result
                .add_error(format!("LineString[{}]", i), "Coordinates must be finite".to_string());
        }
    }

    result
}

fn validate_polygon(rings: &[Vec<[f64; 2]>]) -> ValidationResult {
    let mut result = ValidationResult::valid();

    if rings.is_empty() {
        result.add_error("Polygon".to_string(), "Polygon must have at least one ring".to_string());
        return result;
    }

    let exterior = &rings[0];
    if exterior.len() < 4 {
        result.add_error(
            "Polygon exterior".to_string(),
            format!("Polygon exterior must have at least 4 points, found {}", exterior.len()),
        );
    }

    // Check if exterior is closed
    if let (Some(first), Some(last)) = (exterior.first(), exterior.last()) {
        if first != last {
            result.add_error(
                "Polygon exterior".to_string(),
                "Polygon exterior ring is not closed".to_string(),
            );
        }
    }

    // Check coordinate validity
    for (ring_idx, ring) in rings.iter().enumerate() {
        for (coord_idx, coord) in ring.iter().enumerate() {
            if !is_valid_coord(coord) {
                result.add_error(
                    format!("Polygon ring[{}][{}]", ring_idx, coord_idx),
                    "Coordinates must be finite".to_string(),
                );
            }
        }
    }

    result
}

fn validate_multipoint(coords: &[[f64; 2]]) -> ValidationResult {
    let mut result = ValidationResult::valid();

    for (i, coord) in coords.iter().enumerate() {
        if !is_valid_coord(coord) {
            result
                .add_error(format!("MultiPoint[{}]", i), "Coordinates must be finite".to_string());
        }
    }

    result
}

fn validate_multilinestring(lines: &[Vec<[f64; 2]>]) -> ValidationResult {
    let mut result = ValidationResult::valid();

    for (line_idx, line) in lines.iter().enumerate() {
        if line.len() < 2 {
            result.add_error(
                format!("MultiLineString[{}]", line_idx),
                format!("LineString must have at least 2 points, found {}", line.len()),
            );
        }

        for (coord_idx, coord) in line.iter().enumerate() {
            if !is_valid_coord(coord) {
                result.add_error(
                    format!("MultiLineString[{}][{}]", line_idx, coord_idx),
                    "Coordinates must be finite".to_string(),
                );
            }
        }
    }

    result
}

fn validate_multipolygon(polygons: &[Vec<Vec<[f64; 2]>>]) -> ValidationResult {
    let mut result = ValidationResult::valid();

    for (poly_idx, poly) in polygons.iter().enumerate() {
        if poly.is_empty() {
            result.add_error(
                format!("MultiPolygon[{}]", poly_idx),
                "Polygon must have at least one ring".to_string(),
            );
            continue;
        }

        let exterior = &poly[0];
        if exterior.len() < 4 {
            result.add_error(
                format!("MultiPolygon[{}] exterior", poly_idx),
                format!("Polygon exterior must have at least 4 points, found {}", exterior.len()),
            );
        }

        // Check coordinate validity
        for (ring_idx, ring) in poly.iter().enumerate() {
            for (coord_idx, coord) in ring.iter().enumerate() {
                if !is_valid_coord(coord) {
                    result.add_error(
                        format!("MultiPolygon[{}][{}][{}]", poly_idx, ring_idx, coord_idx),
                        "Coordinates must be finite".to_string(),
                    );
                }
            }
        }
    }

    result
}

/// Fix a geometry if possible (based on validation mode)
///
/// Currently only validates but does not attempt fixes.
/// In strict mode, returns an error if geometry is invalid.
/// In lenient mode, returns the geometry as-is (future: attempt fixes).
pub fn fix_geometry(geometry: &Geometry, mode: ValidityMode) -> Result<Geometry> {
    let validation = validate_geometry(geometry, mode);

    if !validation.is_valid {
        match mode {
            ValidityMode::Strict => {
                let error_msg = validation
                    .errors
                    .iter()
                    .map(|e| format!("{}: {}", e.location, e.reason))
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(GeoragError::FormatError {
                    format: "geometry".into(),
                    message: format!("Invalid geometry: {}", error_msg),
                });
            }
            ValidityMode::Lenient => {
                // In future: attempt to fix common issues like:
                // - Unclosed polygon rings
                // - Duplicate consecutive points
                // For now, just return as-is
            }
        }
    }

    Ok(geometry.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_point() {
        let geom = Geometry::point(115.0, -8.5);
        let result = validate_geometry(&geom, ValidityMode::Strict);
        assert!(result.is_valid);
    }

    #[test]
    fn test_invalid_point_nan() {
        let geom = Geometry::Point { coordinates: [f64::NAN, 0.0] };
        let result = validate_geometry(&geom, ValidityMode::Strict);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_valid_polygon() {
        let geom = Geometry::polygon(vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]]);
        let result = validate_geometry(&geom, ValidityMode::Strict);
        assert!(result.is_valid);
    }

    #[test]
    fn test_invalid_polygon_too_few_points() {
        let geom = Geometry::polygon(vec![vec![[0.0, 0.0], [1.0, 0.0]]]);
        let result = validate_geometry(&geom, ValidityMode::Strict);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_linestring_too_few_points() {
        let geom = Geometry::LineString { coordinates: vec![[0.0, 0.0]] };
        let result = validate_geometry(&geom, ValidityMode::Strict);
        assert!(!result.is_valid);
    }
}
