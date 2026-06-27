#[derive(Debug, Clone, Copy)]
pub(super) struct GridSample {
    pub(super) index: usize,
    pub(super) center_distance_sq: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PointCloudProjectionResult {
    pub points: Vec<[f64; 3]>,
    pub squared_distances: Vec<f64>,
    pub vertex_indices: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PointCloudClosestPair {
    pub vertex_indices: [i64; 2],
    pub squared_distance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PointCloudSelectionObject {
    pub points: Vec<[f64; 3]>,
    pub source_point_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PointCloudPlyDocument {
    pub points: Vec<[f64; 3]>,
    pub normals: Vec<[f64; 3]>,
    pub colors: Vec<[u8; 3]>,
}
