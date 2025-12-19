//! Spatial query operations and predicates

use crate::models::{Geometry, SpatialFilter, SpatialPredicate};
use geo::algorithm::bounding_rect::BoundingRect;
use geo::algorithm::contains::Contains;
use geo::algorithm::intersects::Intersects;
use geo::{Geometry as GeoGeometry, Rect};

/// Evaluate if a geometry satisfies a spatial filter
pub fn evaluate_spatial_filter(geometry: &Geometry, filter: &SpatialFilter) -> bool {
    // If no filter geometry is specified, return true (no spatial constraint)
    let filter_geom = match &filter.geometry {
        Some(g) => g,
        None => return true,
    };

    match filter.predicate {
        SpatialPredicate::Within => evaluate_within(geometry, filter_geom),
        SpatialPredicate::Intersects => evaluate_intersects(geometry, filter_geom),
        SpatialPredicate::Contains => evaluate_contains(geometry, filter_geom),
        SpatialPredicate::BoundingBox => evaluate_bounding_box(geometry, filter_geom),
    }
}

/// Check if geometry is within the filter geometry
fn evaluate_within(geometry: &Geometry, filter: &Geometry) -> bool {
    let geo_geom: GeoGeometry = geometry.clone().into();
    let filter_geom: GeoGeometry = filter.clone().into();
    
    // Within means the geometry is completely inside the filter
    filter_geom.contains(&geo_geom)
}

/// Check if geometry intersects the filter geometry
fn evaluate_intersects(geometry: &Geometry, filter: &Geometry) -> bool {
    let geo_geom: GeoGeometry = geometry.clone().into();
    let filter_geom: GeoGeometry = filter.clone().into();
    
    geo_geom.intersects(&filter_geom)
}

/// Check if geometry contains the filter geometry
fn evaluate_contains(geometry: &Geometry, filter: &Geometry) -> bool {
    let geo_geom: GeoGeometry = geometry.clone().into();
    let filter_geom: GeoGeometry = filter.clone().into();
    
    geo_geom.contains(&filter_geom)
}

/// Check if geometry's bounding box intersects the filter's bounding box
fn evaluate_bounding_box(geometry: &Geometry, filter: &Geometry) -> bool {
    let geo_geom: GeoGeometry = geometry.clone().into();
    let filter_geom: GeoGeometry = filter.clone().into();
    
    // Get bounding rectangles
    let geom_bbox = match geo_geom.bounding_rect() {
        Some(bbox) => bbox,
        None => return false,
    };
    
    let filter_bbox = match filter_geom.bounding_rect() {
        Some(bbox) => bbox,
        None => return false,
    };
    
    // Check if bounding boxes intersect
    bounding_boxes_intersect(&geom_bbox, &filter_bbox)
}

/// Check if two bounding boxes intersect
fn bounding_boxes_intersect(bbox1: &Rect, bbox2: &Rect) -> bool {
    // Two rectangles intersect if they overlap in both x and y dimensions
    let x_overlap = bbox1.min().x <= bbox2.max().x && bbox1.max().x >= bbox2.min().x;
    let y_overlap = bbox1.min().y <= bbox2.max().y && bbox1.max().y >= bbox2.min().y;
    
    x_overlap && y_overlap
}

/// Filter a collection of geometries by a spatial filter
pub fn filter_geometries(
    geometries: &[(Geometry, usize)],
    filter: &SpatialFilter,
) -> Vec<usize> {
    geometries
        .iter()
        .filter_map(|(geom, idx)| {
            if evaluate_spatial_filter(geom, filter) {
                Some(*idx)
            } else {
                None
            }
        })
        .collect()
}

/// Count how many geometries satisfy a spatial filter
pub fn count_spatial_matches(geometries: &[Geometry], filter: &SpatialFilter) -> usize {
    geometries
        .iter()
        .filter(|geom| evaluate_spatial_filter(geom, filter))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Crs;
    use geo::Point;

    #[test]
    fn test_point_within_polygon() {
        // Create a square polygon
        let square = Geometry::Polygon(geo::Polygon::new(
            geo::LineString::from(vec![
                (0.0, 0.0),
                (10.0, 0.0),
                (10.0, 10.0),
                (0.0, 10.0),
                (0.0, 0.0),
            ]),
            vec![],
        ));

        // Point inside the square
        let point_inside = Geometry::Point(Point::new(5.0, 5.0));
        
        // Point outside the square
        let point_outside = Geometry::Point(Point::new(15.0, 15.0));

        let filter = SpatialFilter::new(SpatialPredicate::Within, Crs::wgs84())
            .with_geometry(square);

        assert!(evaluate_spatial_filter(&point_inside, &filter));
        assert!(!evaluate_spatial_filter(&point_outside, &filter));
    }

    #[test]
    fn test_intersects() {
        // Create two overlapping polygons
        let poly1 = Geometry::Polygon(geo::Polygon::new(
            geo::LineString::from(vec![
                (0.0, 0.0),
                (10.0, 0.0),
                (10.0, 10.0),
                (0.0, 10.0),
                (0.0, 0.0),
            ]),
            vec![],
        ));

        let poly2 = Geometry::Polygon(geo::Polygon::new(
            geo::LineString::from(vec![
                (5.0, 5.0),
                (15.0, 5.0),
                (15.0, 15.0),
                (5.0, 15.0),
                (5.0, 5.0),
            ]),
            vec![],
        ));

        let filter = SpatialFilter::new(SpatialPredicate::Intersects, Crs::wgs84())
            .with_geometry(poly1);

        assert!(evaluate_spatial_filter(&poly2, &filter));
    }

    #[test]
    fn test_bounding_box() {
        let point = Geometry::Point(Point::new(5.0, 5.0));
        
        let bbox_geom = Geometry::Polygon(geo::Polygon::new(
            geo::LineString::from(vec![
                (0.0, 0.0),
                (10.0, 0.0),
                (10.0, 10.0),
                (0.0, 10.0),
                (0.0, 0.0),
            ]),
            vec![],
        ));

        let filter = SpatialFilter::new(SpatialPredicate::BoundingBox, Crs::wgs84())
            .with_geometry(bbox_geom);

        assert!(evaluate_spatial_filter(&point, &filter));
    }
}
