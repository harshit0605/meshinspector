#[derive(Debug, Clone, PartialEq)]
pub struct MeshStats {
    pub bbox_min: [f64; 3],
    pub bbox_max: [f64; 3],
    pub bbox_size: [f64; 3],
    pub surface_area_mm2: f64,
    pub volume_mm3: f64,
    pub vertex_count: usize,
    pub face_count: usize,
    pub connected_components: usize,
    pub boundary_edge_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshHealth {
    pub is_closed: bool,
    pub holes_count: usize,
    pub boundary_edge_count: usize,
    pub nonmanifold_edge_count: usize,
    pub self_intersections: Option<usize>,
    pub self_intersections_available: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServiceMeshHealth {
    pub is_closed: bool,
    pub self_intersections: usize,
    pub self_intersection_faces: Vec<usize>,
    pub holes_count: usize,
    pub degenerate_faces: usize,
    pub health_score: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshEditResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub changed_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepairReport {
    pub input_vertex_count: usize,
    pub input_face_count: usize,
    pub output_vertex_count: usize,
    pub output_face_count: usize,
    pub merged_vertices: usize,
    pub removed_degenerate_faces: usize,
    pub removed_unreferenced_vertices: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoleFillReport {
    pub input_holes: usize,
    pub filled_holes: usize,
    pub added_vertices: usize,
    pub added_faces: usize,
    pub new_face_indices: Vec<usize>,
    pub skipped_holes: usize,
}
#[derive(Debug, Clone, PartialEq)]
pub struct HoleFillResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub report: HoleFillReport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicRepairResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub report: RepairReport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThicknessSummary {
    pub min_mm: Option<f64>,
    pub avg_mm: Option<f64>,
    pub max_mm: Option<f64>,
    pub valid_vertex_count: usize,
    pub violation_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MaterialWeightEntry {
    pub volume_mm3: f64,
    pub weight_g: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManufacturabilityReport {
    pub health: MeshHealth,
    pub stats: MeshStats,
    pub ring_measurement: RingMeasurement,
    pub thickness: ThicknessSummary,
    pub regions: Vec<RegionEntry>,
    pub material_weights: Vec<(String, MaterialWeightEntry)>,
    pub recommendations: Vec<String>,
    pub export_ready: bool,
    pub health_score: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DistanceSummary {
    pub min_mm: Option<f64>,
    pub max_mm: Option<f64>,
    pub mean_mm: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VersionCompareSummary {
    pub volume_delta_mm3: f64,
    pub bbox_delta_mm: [f64; 3],
    pub min_signed_distance_mm: Option<f64>,
    pub max_signed_distance_mm: Option<f64>,
    pub mean_signed_distance_mm: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RingMeasurement {
    pub ring_axis: [f64; 3],
    pub ring_axis_confidence: f64,
    pub estimated_ring_size_us: Option<f64>,
    pub inner_diameter_mm: Option<f64>,
    pub band_width_min_mm: Option<f64>,
    pub band_width_max_mm: Option<f64>,
    pub head_height_mm: Option<f64>,
    pub bbox_mm: [f64; 3],
    pub needs_axis_confirmation: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegionEntry {
    pub region_id: String,
    pub label: String,
    pub vertex_indices: Vec<i64>,
    pub coverage_pct: f64,
    pub protected_by_default: bool,
    pub allowed_operations: Vec<String>,
    pub min_thickness_mm: Option<f64>,
    pub avg_thickness_mm: Option<f64>,
    pub violation_count: usize,
    pub centroid_mm: Option<[f64; 3]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrainHolePlan {
    pub center_mm: [f64; 3],
    pub direction: [f64; 3],
    pub radius_mm: f64,
    pub length_mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshArrays {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelDualMeshSettings {
    pub iso_value: f32,
    pub level_set: bool,
    pub adaptivity: f32,
    pub max_faces: usize,
    pub max_vertices: usize,
    pub relax_disoriented_triangles: bool,
}

impl Default for VoxelDualMeshSettings {
    fn default() -> Self {
        Self {
            iso_value: 0.0,
            level_set: false,
            adaptivity: 0.0,
            max_faces: usize::MAX,
            max_vertices: usize::MAX,
            relax_disoriented_triangles: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelMaxDerivSettings {
    pub iters: usize,
    pub sample_points: usize,
    pub degree: usize,
    pub outlier_threshold: f64,
    pub intermediate_smooth_force: f64,
    pub preparation_smooth_force: f64,
    pub smooth_shift_iterations: usize,
    pub final_relax_iterations: usize,
    pub final_relax_force: f64,
}

impl Default for VoxelMaxDerivSettings {
    fn default() -> Self {
        Self {
            iters: 30,
            sample_points: 6,
            degree: 3,
            outlier_threshold: 1.0,
            intermediate_smooth_force: 0.3,
            preparation_smooth_force: 0.1,
            smooth_shift_iterations: 15,
            final_relax_iterations: 15,
            final_relax_force: 0.01,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VoxelMaxDerivResult {
    pub vertices: Vec<[f64; 3]>,
    pub corrected_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoxelSmartMeshResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub corrected_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveHollowResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub achieved_weight_g: f64,
    pub wall_thickness_mm: Option<f64>,
    pub iterations: usize,
    pub warning: Option<String>,
    pub original_weight_g: f64,
    pub target_weight_g: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoxelRebuildReport {
    pub input_vertex_count: usize,
    pub input_face_count: usize,
    pub output_vertex_count: usize,
    pub output_face_count: usize,
    pub input_boundary_edge_count: usize,
    pub output_boundary_edge_count: usize,
    pub input_nonmanifold_edge_count: usize,
    pub output_nonmanifold_edge_count: usize,
    pub input_self_intersections: Option<usize>,
    pub output_self_intersections: Option<usize>,
    pub voxel_size_mm: f64,
    pub offset_mm: f64,
    pub extractor: String,
    pub refine: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoxelRebuildResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub report: VoxelRebuildReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelPathMetric {
    Difference,
    Exponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelPathPlane {
    YZ,
    ZX,
    XY,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelPathOptions {
    pub max_dist_ratio: f32,
    pub plane: VoxelPathPlane,
    pub quarters_mask: u8,
    pub exponent_modifier: f32,
}

impl VoxelPathOptions {
    pub const QUARTER_LEFT_LEFT: u8 = 0b0001;
    pub const QUARTER_LEFT_RIGHT: u8 = 0b0010;
    pub const QUARTER_RIGHT_LEFT: u8 = 0b0100;
    pub const QUARTER_RIGHT_RIGHT: u8 = 0b1000;
    pub const QUARTER_ALL: u8 = 0b1111;
}

impl Default for VoxelPathOptions {
    fn default() -> Self {
        Self {
            max_dist_ratio: 1.5,
            plane: VoxelPathPlane::None,
            quarters_mask: Self::QUARTER_ALL,
            exponent_modifier: -1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VoxelPathResult {
    pub voxel_indices: Vec<usize>,
    pub coordinates: Vec<[usize; 3]>,
    pub total_metric: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoxelPathBuildFourEntry {
    pub quarters_mask: u8,
    pub path: VoxelPathResult,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VoxelPathBuildFourResult {
    pub paths: Vec<VoxelPathBuildFourEntry>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VoxelSliceResult {
    pub width: usize,
    pub height: usize,
    pub values: Vec<f32>,
    pub normalized_values: Vec<f32>,
    pub coordinates: Vec<[usize; 3]>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VoxelLineGraphResult {
    pub axis: usize,
    pub positions: Vec<usize>,
    pub voxel_indices: Vec<usize>,
    pub coordinates: Vec<[usize; 3]>,
    pub values: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VoxelActiveBoxResult {
    pub min_corner: [usize; 3],
    pub dimensions: [usize; 3],
    pub source_indices: Vec<usize>,
    pub coordinates: Vec<[usize; 3]>,
    pub values: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VoxelVolumeRenderDataResult {
    pub dimensions: [usize; 3],
    pub voxel_size: [f64; 3],
    pub source_indices: Vec<usize>,
    pub coordinates: Vec<[usize; 3]>,
    pub values: Vec<f32>,
    pub min_value: f32,
    pub max_value: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoxelVolumeRenderLutResult {
    pub lut_type: String,
    pub alpha_type: String,
    pub alpha_limit: u8,
    pub colors_rgba: Vec<[u8; 4]>,
    pub meshlib_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoxelVolumeRenderRayResult {
    pub color_rgba: [f32; 4],
    pub first_opaque_world: Option<[f64; 3]>,
    pub visited_indices: Vec<usize>,
    pub accepted_indices: Vec<usize>,
    pub meshlib_reference: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelSegmentationOptions {
    pub exponent_modifier: f32,
    pub voxels_expansion: usize,
    pub include_boundary_outside: bool,
}

impl Default for VoxelSegmentationOptions {
    fn default() -> Self {
        Self {
            exponent_modifier: 3000.0,
            voxels_expansion: 25,
            include_boundary_outside: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VoxelSegmentationResult {
    pub min_corner: [usize; 3],
    pub dimensions: [usize; 3],
    pub source_indices: Vec<usize>,
    pub part_indices: Vec<usize>,
    pub selected_coordinates: Vec<[usize; 3]>,
    pub selected_values: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VoxelSegmentationMeshResult {
    pub segmentation: VoxelSegmentationResult,
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VoxelMaskMeshResult {
    pub min_corner: [usize; 3],
    pub dimensions: [usize; 3],
    pub source_indices: Vec<usize>,
    pub part_indices: Vec<usize>,
    pub selected_coordinates: Vec<[usize; 3]>,
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClosestPointsResult {
    pub closest_points: Vec<[f64; 3]>,
    pub distances: Vec<f64>,
    pub face_indices: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RayHit {
    pub face_index: usize,
    pub distance: f64,
    pub point: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct RayHitsResult {
    pub face_indices: Vec<i64>,
    pub distances: Vec<f64>,
    pub points: Vec<[f64; 3]>,
}

mod geometry_error;
pub use geometry_error::GeometryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdfBooleanOperation {
    Union,
    Intersection,
    Difference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelBinaryOperation {
    Union,
    Intersection,
    Difference,
    Max,
    Min,
    Sum,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawVoxelScalarType {
    UInt8,
    Int8,
    UInt16,
    Int16,
    UInt32,
    Int32,
    UInt64,
    Int64,
    Float32,
    Float64,
    Float32_4,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawVoxelParameters {
    pub dimensions: [usize; 3],
    pub voxel_size: [f32; 3],
    pub grid_level_set: bool,
    pub scalar_type: RawVoxelScalarType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawVoxelVolume {
    pub dimensions: [usize; 3],
    pub voxel_size: [f32; 3],
    pub grid_level_set: bool,
    pub scalar_type: RawVoxelScalarType,
    pub values: Vec<f32>,
    pub min: f32,
    pub max: f32,
    pub source_path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TiffVoxelVolume {
    pub dimensions: [usize; 3],
    pub voxel_size: [f32; 3],
    pub grid_level_set: bool,
    pub values: Vec<f32>,
    pub min: f32,
    pub max: f32,
    pub source_path: String,
    pub source_files: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelMeshExtractor {
    Marching,
    Cells,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridMeshExtractionOptions {
    pub voxel_size: f64,
    pub extractor: VoxelMeshExtractor,
    pub refine: bool,
    pub smooth_iterations: i64,
    pub smooth_strength: f64,
    pub projection_iterations: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelMeshOptions {
    pub voxel_size: f64,
    pub padding_mm: Option<f64>,
    pub extractor: VoxelMeshExtractor,
    pub refine: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelBooleanMeshOptions {
    pub voxel_size: f64,
    pub padding_mm: Option<f64>,
    pub origin_phase: [f64; 3],
    pub extractor: VoxelMeshExtractor,
    pub refine: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MarchingTetrahedraResult {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FaceOrientationResult {
    pub faces: Vec<[i64; 3]>,
    pub component_offsets: Vec<usize>,
    pub component_faces: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmoothFalloffOptions {
    pub falloff_mm: f64,
    pub iterations: i64,
    pub strength: f64,
    pub active_threshold: f32,
    pub cutoff_multiplier: f64,
}
