#[derive(Debug, Clone, PartialEq)]
#[rustfmt::skip]
pub enum GeometryError {
    FaceIndexOutOfBounds { face: usize, vertex: i64, vertex_count: usize },
    NegativeFaceIndex { face: usize, vertex: i64 },
    FaceRegionIndexOutOfBounds { index: usize, face_count: usize },
    DirectionTooSmall,
    RayCountMismatch { origins: usize, directions: usize },
    GridTooLarge { shape: [usize; 3] },
    InvalidVoxelSize { voxel_size: f64 },
    InvalidSdfOffset { offset_mm: f64 },
    InvalidWallThickness { wall_thickness_mm: f64 },
    EmptyVoxelValues,
    InvalidVdbPayload { reason: String },
    MeshVerticesLimitExceeded { vertices: usize, limit: usize },
    MeshFacesLimitExceeded { faces: usize, limit: usize },
    InvalidVoxelValue { index: usize, value: f32 },
    MismatchedSdfValueCount { left: usize, right: usize },
    SdfValueCountDoesNotMatchShape { values: usize, shape: [usize; 3] },
    InvalidSdfShape { shape: [usize; 3] },
    InvalidSignMethod { method: String },
    WeightCountDoesNotMatchVertices { weights: usize, vertices: usize },
    ThicknessCountDoesNotMatchVertices { thickness: usize, vertices: usize },
    EmptySeedIndices,
    NegativeSeedIndex { seed: i64 },
    SeedIndexOutOfBounds { seed: i64, vertex_count: usize },
    PreserveIndexOutOfBounds { index: i64, vertex_count: usize },
    InvalidRingFitDiameter { measured_diameter_mm: f64, target_diameter_mm: f64 },
    BrushStrokeCountMismatch { operations: usize, amounts: usize, falloffs: usize, iterations: usize, strengths: usize },
    BrushSeedOffsetCountMismatch { offsets: usize, operations: usize },
    InvalidBrushSeedOffset { offset: i64, seed_count: usize },
    BrushSeedOffsetsNotSorted { previous: i64, next: i64 },
    BrushIndexOffsetCountMismatch { kind: &'static str, offsets: usize, operations: usize },
    InvalidBrushIndexOffset { kind: &'static str, offset: i64, index_count: usize },
    BrushIndexOffsetsNotSorted { kind: &'static str, previous: i64, next: i64 },
    BrushIndexOutOfBounds { kind: &'static str, index: i64, vertex_count: usize },
    UnsupportedBrushOperation { operation: i64 },
    UnknownRegionIds { ids: Vec<String> },
    InvalidRegionOffsets { offsets: usize, regions: usize },
    InvalidRegionOffset { offset: i64, index_count: usize },
    RegionOffsetsNotSorted { previous: i64, next: i64 },
    RegionVertexOutOfBounds { index: i64, vertex_count: usize },
    MissingInnerBandRegion,
    DrainHoleDirectionsUnavailable,
    InvalidDrainHoleSections { sections: usize },
    DrainHolePlanCountMismatch { centers: usize, directions: usize, radii: usize, lengths: usize },
    InvalidAdaptiveHollowInput { field: &'static str, value: f64 },
    InvalidThicknessInput { field: &'static str, value: f64 },
    InvalidMeshEditInput { field: &'static str, value: f64 },
    InvalidSelectionParameter { field: &'static str, value: String },
    InvalidMaxPolygonSubdivisions { max_polygon_subdivisions: usize },
    AdaptiveHollowFailed,
}
impl std::fmt::Display for GeometryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeometryError::FaceIndexOutOfBounds {
                face,
                vertex,
                vertex_count,
            } => write!(
                formatter,
                "face {face} references vertex {vertex}, but vertex count is {vertex_count}"
            ),
            GeometryError::NegativeFaceIndex { face, vertex } => {
                write!(
                    formatter,
                    "face {face} references negative vertex index {vertex}"
                )
            }
            GeometryError::FaceRegionIndexOutOfBounds { index, face_count } => write!(
                formatter,
                "face region index {index} is out of range for {face_count} faces"
            ),
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
            GeometryError::EmptyVoxelValues => {
                write!(formatter, "voxel values must not be empty")
            }
            GeometryError::InvalidVdbPayload { reason } => {
                write!(formatter, "invalid OpenVDB voxel payload: {reason}")
            }
            GeometryError::MeshVerticesLimitExceeded { .. } => {
                write!(formatter, "Vertices number limit exceeded.")
            }
            GeometryError::MeshFacesLimitExceeded { .. } => {
                write!(formatter, "Triangles number limit exceeded.")
            }
            GeometryError::InvalidVoxelValue { index, value } => {
                write!(
                    formatter,
                    "voxel value at index {index} must not be NaN, got {value}"
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
            GeometryError::InvalidRingFitDiameter {
                measured_diameter_mm,
                target_diameter_mm,
            } => {
                write!(
                    formatter,
                    "ring fit requires positive measured ({measured_diameter_mm}) and target ({target_diameter_mm}) diameters"
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
            GeometryError::InvalidMeshEditInput { field, value } => write!(
                formatter,
                "mesh edit {field} must be finite and valid, got {value}"
            ),
            GeometryError::InvalidSelectionParameter { field, value } => {
                write!(formatter, "selection {field} is invalid: {value}")
            }
            GeometryError::InvalidMaxPolygonSubdivisions {
                max_polygon_subdivisions,
            } => {
                write!(
                    formatter,
                    "max_polygon_subdivisions must be at least 2, got {max_polygon_subdivisions}"
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
