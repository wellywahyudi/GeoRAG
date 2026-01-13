use crate::models::{Geometry, SpatialFilter, SpatialPredicate};
use geo::algorithm::bounding_rect::BoundingRect;
use geo::algorithm::centroid::Centroid;
use geo::algorithm::contains::Contains;
use geo::algorithm::intersects::Intersects;
use geo::{Distance, Geometry as GeoGeometry, Haversine, Point, Rect};

/// Evaluate if a geometry satisfies a spatial filter
pub fn evaluate_spatial_filter(geometry: &Geometry, filter: &SpatialFilter) -> bool {
    // DWithin is special: it can work without a filter geometry (using distance from origin)
    if filter.predicate == SpatialPredicate::DWithin {
        return evaluate_dwithin(geometry, filter);
    }

    // For other predicates, if no filter geometry is specified, return true (no spatial constraint)
    let filter_geom = match &filter.geometry {
        Some(g) => g,
        None => return true,
    };

    match filter.predicate {
        SpatialPredicate::Within => evaluate_within(geometry, filter_geom),
        SpatialPredicate::Intersects => evaluate_intersects(geometry, filter_geom),
        SpatialPredicate::Contains => evaluate_contains(geometry, filter_geom),
        SpatialPredicate::BoundingBox => evaluate_bounding_box(geometry, filter_geom),
        SpatialPredicate::DWithin => unreachable!(),
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

/// Calculate geodesic distance between two geometries in meters
/// Returns None if centroids cannot be computed (e.g., empty geometries).
pub fn geodesic_distance(geom1: &Geometry, geom2: &Geometry) -> Option<f64> {
    let geo1: GeoGeometry = geom1.clone().into();
    let geo2: GeoGeometry = geom2.clone().into();

    match (&geo1, &geo2) {
        // Exact point-to-point distance using Haversine
        (GeoGeometry::Point(p1), GeoGeometry::Point(p2)) => Some(Haversine.distance(*p1, *p2)),
        // For other geometries, compute centroids and measure distance between them
        _ => {
            let c1: Point = geo1.centroid()?;
            let c2: Point = geo2.centroid()?;
            Some(Haversine.distance(c1, c2))
        }
    }
}

/// Evaluate DWithin predicate (distance within threshold)
/// Requires both a filter geometry and a distance to be specified.
fn evaluate_dwithin(geometry: &Geometry, filter: &SpatialFilter) -> bool {
    // DWithin requires a filter geometry
    let filter_geom = match &filter.geometry {
        Some(g) => g,
        None => return false,
    };

    // DWithin requires a distance threshold
    let distance = match &filter.distance {
        Some(d) => d,
        None => return false,
    };

    let threshold_meters = distance.to_meters();

    // Calculate geodesic distance and compare to threshold
    match geodesic_distance(geometry, filter_geom) {
        Some(dist) => dist <= threshold_meters,
        None => false,
    }
}

/// Filter a collection of geometries by a spatial filter
pub fn filter_geometries(geometries: &[(Geometry, usize)], filter: &SpatialFilter) -> Vec<usize> {
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
    geometries.iter().filter(|geom| evaluate_spatial_filter(geom, filter)).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Crs;
    use geo::Point;

    #[test]
    fn test_point_within_polygon() {
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

        let filter =
            SpatialFilter::new(SpatialPredicate::Within, Crs::wgs84()).with_geometry(square);

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

        let filter =
            SpatialFilter::new(SpatialPredicate::Intersects, Crs::wgs84()).with_geometry(poly1);

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

    #[test]
    fn test_dwithin_point_to_point() {
        use crate::models::{Distance, DistanceUnit};

        // Two points in Bali, approximately 1.1km apart
        // Ubud center: 115.2625, -8.5069
        // Point ~1.1km east: 115.2725, -8.5069 (0.01 degree ≈ 1.1km at equator)
        let center_point = Geometry::Point(Point::new(115.2625, -8.5069));
        let nearby_point = Geometry::Point(Point::new(115.2725, -8.5069));
        let far_point = Geometry::Point(Point::new(115.3625, -8.5069)); // ~11km away

        // Test: nearby point should be within 2km
        let filter_2km = SpatialFilter::new(SpatialPredicate::DWithin, Crs::wgs84())
            .with_geometry(center_point.clone())
            .with_distance(Distance::new(2000.0, DistanceUnit::Meters));

        assert!(
            evaluate_spatial_filter(&nearby_point, &filter_2km),
            "Nearby point should be within 2km"
        );

        // Test: nearby point should NOT be within 500m
        let filter_500m = SpatialFilter::new(SpatialPredicate::DWithin, Crs::wgs84())
            .with_geometry(center_point.clone())
            .with_distance(Distance::new(500.0, DistanceUnit::Meters));

        assert!(
            !evaluate_spatial_filter(&nearby_point, &filter_500m),
            "Nearby point should NOT be within 500m"
        );

        // Test: far point should not be within 2km
        assert!(
            !evaluate_spatial_filter(&far_point, &filter_2km),
            "Far point should NOT be within 2km"
        );
    }

    #[test]
    fn test_dwithin_with_kilometers() {
        use crate::models::{Distance, DistanceUnit};

        let point1 = Geometry::Point(Point::new(115.2625, -8.5069));
        let point2 = Geometry::Point(Point::new(115.3625, -8.5069)); // ~11km away

        // Test with kilometers unit
        let filter_15km = SpatialFilter::new(SpatialPredicate::DWithin, Crs::wgs84())
            .with_geometry(point1.clone())
            .with_distance(Distance::new(15.0, DistanceUnit::Kilometers));

        assert!(evaluate_spatial_filter(&point2, &filter_15km), "Point should be within 15km");

        let filter_5km = SpatialFilter::new(SpatialPredicate::DWithin, Crs::wgs84())
            .with_geometry(point1)
            .with_distance(Distance::new(5.0, DistanceUnit::Kilometers));

        assert!(!evaluate_spatial_filter(&point2, &filter_5km), "Point should NOT be within 5km");
    }

    #[test]
    fn test_dwithin_polygon_to_point() {
        use crate::models::{Distance, DistanceUnit};

        // A small parcel polygon in Bali
        let parcel = Geometry::Polygon(geo::Polygon::new(
            geo::LineString::from(vec![
                (115.26, -8.50),
                (115.27, -8.50),
                (115.27, -8.51),
                (115.26, -8.51),
                (115.26, -8.50),
            ]),
            vec![],
        ));

        // Point near the parcel centroid
        let near_point = Geometry::Point(Point::new(115.265, -8.505));
        // Point far from the parcel
        let far_point = Geometry::Point(Point::new(115.40, -8.50)); // ~15km away

        let filter = SpatialFilter::new(SpatialPredicate::DWithin, Crs::wgs84())
            .with_geometry(parcel)
            .with_distance(Distance::new(5.0, DistanceUnit::Kilometers));

        assert!(
            evaluate_spatial_filter(&near_point, &filter),
            "Near point should be within 5km of parcel centroid"
        );
        assert!(
            !evaluate_spatial_filter(&far_point, &filter),
            "Far point should NOT be within 5km of parcel centroid"
        );
    }

    #[test]
    fn test_dwithin_requires_distance() {
        // DWithin without distance should return false
        let point1 = Geometry::Point(Point::new(0.0, 0.0));
        let point2 = Geometry::Point(Point::new(0.0, 0.0));

        let filter_no_distance =
            SpatialFilter::new(SpatialPredicate::DWithin, Crs::wgs84()).with_geometry(point1);

        assert!(
            !evaluate_spatial_filter(&point2, &filter_no_distance),
            "DWithin without distance should return false"
        );
    }

    #[test]
    fn test_dwithin_requires_geometry() {
        use crate::models::{Distance, DistanceUnit};

        // DWithin without geometry should return false
        let point = Geometry::Point(Point::new(0.0, 0.0));

        let filter_no_geometry = SpatialFilter::new(SpatialPredicate::DWithin, Crs::wgs84())
            .with_distance(Distance::new(1000.0, DistanceUnit::Meters));

        assert!(
            !evaluate_spatial_filter(&point, &filter_no_geometry),
            "DWithin without geometry should return false"
        );
    }

    #[test]
    fn test_geodesic_distance_accuracy() {
        // Test with a known distance
        // Paris (2.3522, 48.8566) to London (0.1276, 51.5074) ≈ 344km
        let paris = Geometry::Point(Point::new(2.3522, 48.8566));
        let london = Geometry::Point(Point::new(-0.1276, 51.5074));

        let distance = geodesic_distance(&paris, &london).expect("Should compute distance");

        // Distance should be approximately 344km (± 5km tolerance)
        assert!(
            distance > 339_000.0 && distance < 349_000.0,
            "Paris-London distance {} should be ~344km",
            distance
        );
    }

    #[test]
    fn test_geodesic_distance_same_point() {
        let point = Geometry::Point(Point::new(115.0, -8.0));

        let distance = geodesic_distance(&point, &point).expect("Should compute distance");

        assert!(distance < 0.001, "Distance from point to itself should be ~0, got {}", distance);
    }
}
