use crate::geo::models::{to_geo_geometry, Geometry, SpatialFilter};
use crate::geo::spatial::evaluate_spatial_filter;
use rstar::{RTree, RTreeObject, AABB};

/// Indexed geometry with ID
#[derive(Debug, Clone, PartialEq)]
pub struct IndexedGeometry {
    /// Unique identifier for this geometry
    pub id: usize,

    /// The geometry itself
    pub geometry: Geometry,

    /// Bounding box for spatial indexing
    envelope: AABB<[f64; 2]>,
}

impl IndexedGeometry {
    /// Create a new indexed geometry
    pub fn new(id: usize, geometry: Geometry) -> Self {
        let envelope = Self::compute_envelope(&geometry);
        Self { id, geometry, envelope }
    }

    /// Compute the bounding box (envelope) for a geometry
    fn compute_envelope(geometry: &Geometry) -> AABB<[f64; 2]> {
        use geo::algorithm::bounding_rect::BoundingRect;

        let geo_geom = to_geo_geometry(geometry);

        match geo_geom.bounding_rect() {
            Some(rect) => {
                let min = rect.min();
                let max = rect.max();
                AABB::from_corners([min.x, min.y], [max.x, max.y])
            }
            None => {
                // Empty/point geometries have no bounding rect. Use origin as a
                // degenerate envelope. Note: this may cause false positives in
                // queries near (0,0) for geographic CRS. Consider filtering by
                // geometry validity upstream.
                AABB::from_point([0.0, 0.0])
            }
        }
    }
}

impl RTreeObject for IndexedGeometry {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.envelope
    }
}

/// Spatial index for efficient geometric queries
pub struct SpatialIndex {
    tree: RTree<IndexedGeometry>,
}

impl SpatialIndex {
    /// Create a new empty spatial index
    pub fn new() -> Self {
        Self { tree: RTree::new() }
    }

    /// Create a spatial index from a collection of geometries
    pub fn from_geometries(geometries: Vec<(usize, Geometry)>) -> Self {
        let indexed: Vec<IndexedGeometry> = geometries
            .into_iter()
            .map(|(id, geom)| IndexedGeometry::new(id, geom))
            .collect();

        Self { tree: RTree::bulk_load(indexed) }
    }

    /// Insert a geometry into the index
    pub fn insert(&mut self, id: usize, geometry: Geometry) {
        let indexed = IndexedGeometry::new(id, geometry);
        self.tree.insert(indexed);
    }

    /// Remove a geometry from the index by ID
    pub fn remove(&mut self, id: usize) -> Option<Geometry> {
        // Find the geometry with this ID
        let to_remove = self.tree.iter().find(|g| g.id == id).cloned();

        if let Some(indexed) = to_remove {
            self.tree.remove(&indexed);
            Some(indexed.geometry)
        } else {
            None
        }
    }

    /// Query geometries within a bounding box
    pub fn query_bbox(&self, min: [f64; 2], max: [f64; 2]) -> Vec<&IndexedGeometry> {
        let bbox = AABB::from_corners(min, max);
        self.tree.locate_in_envelope(&bbox).collect()
    }

    /// Query geometries near a point within a distance
    pub fn query_nearest(&self, point: [f64; 2], max_distance: f64) -> Vec<&IndexedGeometry> {
        // Use bounding box query as approximation
        let min = [point[0] - max_distance, point[1] - max_distance];
        let max = [point[0] + max_distance, point[1] + max_distance];
        self.query_bbox(min, max)
    }

    /// Find the k nearest geometries to a point
    pub fn query_k_nearest(&self, point: [f64; 2], k: usize) -> Vec<&IndexedGeometry> {
        // Get all geometries and sort by distance
        let mut all: Vec<_> = self.tree.iter().collect();
        all.sort_by(|a, b| {
            let dist_a = self.distance_to_envelope(&a.envelope, point);
            let dist_b = self.distance_to_envelope(&b.envelope, point);
            dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
        });
        all.into_iter().take(k).collect()
    }

    /// Calculate distance from a point to an envelope (bounding box)
    fn distance_to_envelope(&self, envelope: &AABB<[f64; 2]>, point: [f64; 2]) -> f64 {
        let lower = envelope.lower();
        let upper = envelope.upper();

        let dx = if point[0] < lower[0] {
            lower[0] - point[0]
        } else if point[0] > upper[0] {
            point[0] - upper[0]
        } else {
            0.0
        };

        let dy = if point[1] < lower[1] {
            lower[1] - point[1]
        } else if point[1] > upper[1] {
            point[1] - upper[1]
        } else {
            0.0
        };

        (dx * dx + dy * dy).sqrt()
    }

    /// Query geometries using a spatial filter
    pub fn query_filter(&self, filter: &SpatialFilter) -> Vec<usize> {
        // First, get candidates using bounding box query
        let candidates = if let Some(filter_geom) = &filter.geometry {
            let geo_geom = to_geo_geometry(filter_geom);

            use geo::algorithm::bounding_rect::BoundingRect;
            if let Some(bbox) = geo_geom.bounding_rect() {
                let min = bbox.min();
                let max = bbox.max();
                self.query_bbox([min.x, min.y], [max.x, max.y])
            } else {
                // No bounding box, return all geometries
                self.tree.iter().collect()
            }
        } else {
            // No filter geometry, return all
            self.tree.iter().collect()
        };

        // Then, apply the actual spatial predicate
        candidates
            .into_iter()
            .filter(|indexed| evaluate_spatial_filter(&indexed.geometry, filter))
            .map(|indexed| indexed.id)
            .collect()
    }

    /// Get the total number of geometries in the index
    pub fn len(&self) -> usize {
        self.tree.size()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.tree.size() == 0
    }

    /// Get all geometry IDs in the index
    pub fn all_ids(&self) -> Vec<usize> {
        self.tree.iter().map(|g| g.id).collect()
    }

    /// Clear the index
    pub fn clear(&mut self) {
        self.tree = RTree::new();
    }
}

impl Default for SpatialIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating spatial indexes
pub struct SpatialIndexBuilder {
    geometries: Vec<(usize, Geometry)>,
}

impl SpatialIndexBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self { geometries: Vec::new() }
    }

    /// Add a geometry to the builder
    pub fn add(mut self, id: usize, geometry: Geometry) -> Self {
        self.geometries.push((id, geometry));
        self
    }

    /// Add multiple geometries to the builder
    pub fn add_many(mut self, geometries: Vec<(usize, Geometry)>) -> Self {
        self.geometries.extend(geometries);
        self
    }

    /// Build the spatial index
    pub fn build(self) -> SpatialIndex {
        SpatialIndex::from_geometries(self.geometries)
    }
}

impl Default for SpatialIndexBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::models::SpatialPredicate;

    #[test]
    fn test_spatial_index_creation() {
        let index = SpatialIndex::new();
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_spatial_index_insert() {
        let mut index = SpatialIndex::new();
        let point = Geometry::point(1.0, 2.0);

        index.insert(1, point);

        assert_eq!(index.len(), 1);
        assert!(!index.is_empty());
    }

    #[test]
    fn test_spatial_index_remove() {
        let mut index = SpatialIndex::new();
        let point = Geometry::point(1.0, 2.0);

        index.insert(1, point.clone());
        assert_eq!(index.len(), 1);

        let removed = index.remove(1);
        assert!(removed.is_some());
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn test_spatial_index_bbox_query() {
        let mut index = SpatialIndex::new();

        // Add points in a grid
        index.insert(1, Geometry::point(0.0, 0.0));
        index.insert(2, Geometry::point(5.0, 5.0));
        index.insert(3, Geometry::point(10.0, 10.0));

        // Query a bounding box that should contain only the first two points
        let results = index.query_bbox([0.0, 0.0], [6.0, 6.0]);

        assert_eq!(results.len(), 2);
        let ids: Vec<usize> = results.iter().map(|g| g.id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
        assert!(!ids.contains(&3));
    }

    #[test]
    fn test_spatial_index_nearest() {
        let mut index = SpatialIndex::new();

        index.insert(1, Geometry::point(0.0, 0.0));
        index.insert(2, Geometry::point(5.0, 5.0));
        index.insert(3, Geometry::point(10.0, 10.0));

        // Find nearest to (1, 1)
        let results = index.query_k_nearest([1.0, 1.0], 1);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1);
    }

    #[test]
    fn test_spatial_index_builder() {
        let point1 = Geometry::point(0.0, 0.0);
        let point2 = Geometry::point(5.0, 5.0);

        let index = SpatialIndexBuilder::new().add(1, point1).add(2, point2).build();

        assert_eq!(index.len(), 2);
    }

    #[test]
    fn test_spatial_index_filter() {
        let mut index = SpatialIndex::new();

        // Create a square polygon using coordinate arrays
        let square = Geometry::polygon(vec![vec![
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 10.0],
            [0.0, 10.0],
            [0.0, 0.0],
        ]]);

        // Add points inside and outside the square
        index.insert(1, Geometry::point(5.0, 5.0)); // Inside
        index.insert(2, Geometry::point(15.0, 15.0)); // Outside

        // Query with Within predicate
        let filter = SpatialFilter::new(SpatialPredicate::Within).geometry(square);

        let results = index.query_filter(&filter);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], 1);
    }

    #[test]
    fn test_spatial_index_clear() {
        let mut index = SpatialIndex::new();
        index.insert(1, Geometry::point(0.0, 0.0));
        index.insert(2, Geometry::point(5.0, 5.0));

        assert_eq!(index.len(), 2);

        index.clear();

        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }
}
