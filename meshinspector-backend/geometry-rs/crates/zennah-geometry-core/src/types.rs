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

#[derive(Debug, Clone, PartialEq)]
pub enum GeometryError {
    FaceIndexOutOfBounds {
        face: usize,
        vertex: i64,
        vertex_count: usize,
    },
    NegativeFaceIndex {
        face: usize,
        vertex: i64,
    },
    DirectionTooSmall,
    RayCountMismatch {
        origins: usize,
        directions: usize,
    },
    GridTooLarge {
        shape: [usize; 3],
    },
    InvalidVoxelSize {
        voxel_size: f64,
    },
    InvalidSdfOffset {
        offset_mm: f64,
    },
    InvalidWallThickness {
        wall_thickness_mm: f64,
    },
    MismatchedSdfValueCount {
        left: usize,
        right: usize,
    },
    SdfValueCountDoesNotMatchShape {
        values: usize,
        shape: [usize; 3],
    },
    InvalidSdfShape {
        shape: [usize; 3],
    },
    InvalidSignMethod {
        method: String,
    },
    WeightCountDoesNotMatchVertices {
        weights: usize,
        vertices: usize,
    },
    ThicknessCountDoesNotMatchVertices {
        thickness: usize,
        vertices: usize,
    },
    EmptySeedIndices,
    NegativeSeedIndex {
        seed: i64,
    },
    SeedIndexOutOfBounds {
        seed: i64,
        vertex_count: usize,
    },
    PreserveIndexOutOfBounds {
        index: i64,
        vertex_count: usize,
    },
    BrushStrokeCountMismatch {
        operations: usize,
        amounts: usize,
        falloffs: usize,
        iterations: usize,
        strengths: usize,
    },
    BrushSeedOffsetCountMismatch {
        offsets: usize,
        operations: usize,
    },
    InvalidBrushSeedOffset {
        offset: i64,
        seed_count: usize,
    },
    BrushSeedOffsetsNotSorted {
        previous: i64,
        next: i64,
    },
    BrushIndexOffsetCountMismatch {
        kind: &'static str,
        offsets: usize,
        operations: usize,
    },
    InvalidBrushIndexOffset {
        kind: &'static str,
        offset: i64,
        index_count: usize,
    },
    BrushIndexOffsetsNotSorted {
        kind: &'static str,
        previous: i64,
        next: i64,
    },
    BrushIndexOutOfBounds {
        kind: &'static str,
        index: i64,
        vertex_count: usize,
    },
    UnsupportedBrushOperation {
        operation: i64,
    },
    UnknownRegionIds {
        ids: Vec<String>,
    },
    InvalidRegionOffsets {
        offsets: usize,
        regions: usize,
    },
    InvalidRegionOffset {
        offset: i64,
        index_count: usize,
    },
    RegionOffsetsNotSorted {
        previous: i64,
        next: i64,
    },
    RegionVertexOutOfBounds {
        index: i64,
        vertex_count: usize,
    },
    MissingInnerBandRegion,
    DrainHoleDirectionsUnavailable,
    InvalidDrainHoleSections {
        sections: usize,
    },
    DrainHolePlanCountMismatch {
        centers: usize,
        directions: usize,
        radii: usize,
        lengths: usize,
    },
    InvalidAdaptiveHollowInput {
        field: &'static str,
        value: f64,
    },
    InvalidThicknessInput {
        field: &'static str,
        value: f64,
    },
    AdaptiveHollowFailed,
}

impl std::fmt::Display for GeometryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeometryError::FaceIndexOutOfBounds {
                face,
                vertex,
                vertex_count,
            } => {
                write!(
                    formatter,
                    "face {face} references vertex {vertex}, but vertex count is {vertex_count}"
                )
            }
            GeometryError::NegativeFaceIndex { face, vertex } => {
                write!(
                    formatter,
                    "face {face} references negative vertex index {vertex}"
                )
            }
            GeometryError::DirectionTooSmall => {
                write!(formatter, "ray direction vector magnitude is too small")
            }
            GeometryError::RayCountMismatch {
                origins,
                directions,
            } => {
                write!(
                    formatter,
                    "ray origin count {origins} does not match direction count {directions}"
                )
            }
            GeometryError::GridTooLarge { shape } => {
                write!(formatter, "SDF grid shape {shape:?} is too large")
            }
            GeometryError::InvalidVoxelSize { voxel_size } => {
                write!(
                    formatter,
                    "voxel size must be positive and finite, got {voxel_size}"
                )
            }
            GeometryError::InvalidSdfOffset { offset_mm } => {
                write!(formatter, "SDF offset must be finite, got {offset_mm}")
            }
            GeometryError::InvalidWallThickness { wall_thickness_mm } => {
                write!(
                    formatter,
                    "wall thickness must be positive and finite, got {wall_thickness_mm}"
                )
            }
            GeometryError::MismatchedSdfValueCount { left, right } => {
                write!(
                    formatter,
                    "SDF value arrays must have the same length, got {left} and {right}"
                )
            }
            GeometryError::SdfValueCountDoesNotMatchShape { values, shape } => {
                write!(
                    formatter,
                    "SDF value count {values} does not match grid shape {shape:?}"
                )
            }
            GeometryError::InvalidSdfShape { shape } => {
                write!(
                    formatter,
                    "SDF grid shape {shape:?} must have at least two samples on each axis"
                )
            }
            GeometryError::InvalidSignMethod { method } => {
                write!(
                    formatter,
                    "sign_method must be 'auto', 'winding', 'ray', or 'unsigned', got {method}"
                )
            }
            GeometryError::WeightCountDoesNotMatchVertices { weights, vertices } => {
                write!(
                    formatter,
                    "weight count {weights} does not match vertex count {vertices}"
                )
            }
            GeometryError::ThicknessCountDoesNotMatchVertices {
                thickness,
                vertices,
            } => {
                write!(
                    formatter,
                    "thickness count {thickness} does not match vertex count {vertices}"
                )
            }
            GeometryError::EmptySeedIndices => {
                write!(formatter, "seed indices must not be empty")
            }
            GeometryError::NegativeSeedIndex { seed } => {
                write!(formatter, "seed index {seed} is negative")
            }
            GeometryError::SeedIndexOutOfBounds { seed, vertex_count } => {
                write!(
                    formatter,
                    "seed index {seed} is outside vertex count {vertex_count}"
                )
            }
            GeometryError::PreserveIndexOutOfBounds {
                index,
                vertex_count,
            } => {
                write!(
                    formatter,
                    "preserve index {index} is outside vertex count {vertex_count}"
                )
            }
            GeometryError::BrushStrokeCountMismatch {
                operations,
                amounts,
                falloffs,
                iterations,
                strengths,
            } => {
                write!(
                    formatter,
                    "brush stroke parameter counts do not match: operations={operations}, amounts={amounts}, falloffs={falloffs}, iterations={iterations}, strengths={strengths}"
                )
            }
            GeometryError::BrushSeedOffsetCountMismatch {
                offsets,
                operations,
            } => {
                write!(
                    formatter,
                    "brush seed offset count {offsets} must equal operation count {operations} plus one"
                )
            }
            GeometryError::InvalidBrushSeedOffset { offset, seed_count } => {
                write!(
                    formatter,
                    "brush seed offset {offset} is outside flattened seed count {seed_count}"
                )
            }
            GeometryError::BrushSeedOffsetsNotSorted { previous, next } => {
                write!(
                    formatter,
                    "brush seed offsets must be sorted, got {previous} before {next}"
                )
            }
            GeometryError::BrushIndexOffsetCountMismatch {
                kind,
                offsets,
                operations,
            } => {
                write!(
                    formatter,
                    "brush {kind} offset count {offsets} must equal operation count {operations} plus one"
                )
            }
            GeometryError::InvalidBrushIndexOffset {
                kind,
                offset,
                index_count,
            } => {
                write!(
                    formatter,
                    "brush {kind} offset {offset} is outside flattened index count {index_count}"
                )
            }
            GeometryError::BrushIndexOffsetsNotSorted {
                kind,
                previous,
                next,
            } => {
                write!(
                    formatter,
                    "brush {kind} offsets must be sorted, got {previous} before {next}"
                )
            }
            GeometryError::BrushIndexOutOfBounds {
                kind,
                index,
                vertex_count,
            } => {
                write!(
                    formatter,
                    "brush {kind} index {index} is outside vertex count {vertex_count}"
                )
            }
            GeometryError::UnsupportedBrushOperation { operation } => {
                write!(formatter, "unsupported brush operation code {operation}")
            }
            GeometryError::UnknownRegionIds { ids } => {
                write!(formatter, "unknown region id(s): {}", ids.join(", "))
            }
            GeometryError::InvalidRegionOffsets { offsets, regions } => {
                write!(
                    formatter,
                    "region offset count {offsets} must equal region count {regions} plus one"
                )
            }
            GeometryError::InvalidRegionOffset {
                offset,
                index_count,
            } => {
                write!(
                    formatter,
                    "region offset {offset} is outside flattened vertex index count {index_count}"
                )
            }
            GeometryError::RegionOffsetsNotSorted { previous, next } => {
                write!(
                    formatter,
                    "region offsets must be sorted, got {previous} before {next}"
                )
            }
            GeometryError::RegionVertexOutOfBounds {
                index,
                vertex_count,
            } => {
                write!(
                    formatter,
                    "region vertex index {index} is outside vertex count {vertex_count}"
                )
            }
            GeometryError::MissingInnerBandRegion => {
                write!(
                    formatter,
                    "Drain-hole planning requires inner_band region data"
                )
            }
            GeometryError::DrainHoleDirectionsUnavailable => {
                write!(
                    formatter,
                    "Unable to determine radial directions for drain holes"
                )
            }
            GeometryError::InvalidDrainHoleSections { sections } => {
                write!(formatter, "sections must be at least 8, got {sections}")
            }
            GeometryError::DrainHolePlanCountMismatch {
                centers,
                directions,
                radii,
                lengths,
            } => {
                write!(
                    formatter,
                    "drain-hole plan arrays must have matching lengths, got centers={centers}, directions={directions}, radii={radii}, lengths={lengths}"
                )
            }
            GeometryError::InvalidAdaptiveHollowInput { field, value } => {
                write!(
                    formatter,
                    "adaptive hollow {field} must be positive and finite, got {value}"
                )
            }
            GeometryError::InvalidThicknessInput { field, value } => {
                write!(
                    formatter,
                    "thickness {field} must be positive and finite, got {value}"
                )
            }
            GeometryError::AdaptiveHollowFailed => {
                write!(
                    formatter,
                    "adaptive hollowing failed to produce a shell mesh"
                )
            }
        }
    }
}

impl std::error::Error for GeometryError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdfBooleanOperation {
    Union,
    Intersection,
    Difference,
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
