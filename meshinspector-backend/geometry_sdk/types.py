"""Core SDK data structures."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Literal

import numpy as np


UnitSystem = Literal["mm"]
MaterialName = Literal["gold_24k", "gold_22k", "gold_18k", "gold_14k", "gold_10k", "silver_925", "platinum"]
BrushOperation = Literal["thicken", "scoop", "smooth"]


def _optional_index_array(indices: np.ndarray | None) -> np.ndarray | None:
    if indices is None:
        return None
    values = np.unique(np.asarray(indices, dtype=np.int64).reshape(-1))
    if np.any(values < 0):
        raise ValueError("indices must be non-negative")
    return values


@dataclass(slots=True)
class MeshDocument:
    """SDK-owned triangular mesh container.

    The SDK stores geometry as NumPy arrays so algorithms can be developed
    independently of MeshLib and trimesh object models.
    """

    vertices: np.ndarray
    faces: np.ndarray
    unit: UnitSystem = "mm"
    metadata: dict[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        vertices = np.asarray(self.vertices, dtype=np.float64)
        faces = np.asarray(self.faces, dtype=np.int64)
        if vertices.ndim != 2 or vertices.shape[1] != 3:
            raise ValueError("vertices must have shape (n, 3)")
        if faces.ndim != 2 or faces.shape[1] != 3:
            raise ValueError("faces must have shape (m, 3)")
        if faces.size and (faces.min() < 0 or faces.max() >= len(vertices)):
            raise ValueError("faces reference vertex indices outside vertices")
        self.vertices = np.ascontiguousarray(vertices)
        self.faces = np.ascontiguousarray(faces)

    def copy(self, *, vertices: np.ndarray | None = None, faces: np.ndarray | None = None) -> "MeshDocument":
        return MeshDocument(
            vertices=np.array(self.vertices if vertices is None else vertices, copy=True),
            faces=np.array(self.faces if faces is None else faces, copy=True),
            unit=self.unit,
            metadata=dict(self.metadata),
        )

    @property
    def vertex_count(self) -> int:
        return int(self.vertices.shape[0])

    @property
    def face_count(self) -> int:
        return int(self.faces.shape[0])


@dataclass(slots=True)
class ExactBooleanMeshResult:
    mesh: MeshDocument
    operation: str
    diagnostics: dict[str, Any]


@dataclass(slots=True)
class RingFitResult:
    mesh: MeshDocument
    applied_uniform_fallback: bool
    scale_factor: float


@dataclass(slots=True)
class MeshCollisionFacePair:
    first_face: int
    second_face: int
    intersection_count: int


@dataclass(slots=True)
class MeshCollisionResult:
    colliding: bool
    pair_count: int
    first_face_indices: list[int]
    second_face_indices: list[int]
    pairs: list[MeshCollisionFacePair]
    truncated: bool = False
    metadata: dict[str, Any] = field(default_factory=dict)


@dataclass(slots=True)
class SubdivideMeshResult:
    mesh: MeshDocument
    splits_done: int
    region_faces: np.ndarray
    region_face_count: int


@dataclass(slots=True)
class DecimateMeshResult:
    mesh: MeshDocument
    verts_deleted: int
    faces_deleted: int
    error_introduced: float
    cancelled: bool


@dataclass(slots=True)
class MeshStats:
    bbox_min: tuple[float, float, float]
    bbox_max: tuple[float, float, float]
    bbox_size: tuple[float, float, float]
    surface_area_mm2: float
    volume_mm3: float
    vertex_count: int
    face_count: int
    connected_components: int
    boundary_edge_count: int


@dataclass(slots=True)
class BrushStroke:
    operation: BrushOperation
    seed_indices: np.ndarray
    amount_mm: float = 0.0
    falloff_mm: float = 1.5
    iterations: int = 1
    strength: float = 0.5
    mask_indices: np.ndarray | None = None
    protected_indices: np.ndarray | None = None

    def __post_init__(self) -> None:
        if self.operation not in {"thicken", "scoop", "smooth"}:
            raise ValueError("operation must be 'thicken', 'scoop', or 'smooth'")
        seed_indices = np.unique(np.asarray(self.seed_indices, dtype=np.int64).reshape(-1))
        if seed_indices.size == 0:
            raise ValueError("seed_indices must not be empty")
        if np.any(seed_indices < 0):
            raise ValueError("seed_indices must be non-negative")
        self.seed_indices = seed_indices
        self.amount_mm = float(self.amount_mm)
        self.falloff_mm = float(self.falloff_mm)
        self.iterations = max(1, int(self.iterations))
        self.strength = float(np.clip(self.strength, 0.0, 1.0))
        self.mask_indices = _optional_index_array(self.mask_indices)
        self.protected_indices = _optional_index_array(self.protected_indices)


@dataclass(slots=True)
class MeshHealth:
    is_closed: bool
    holes_count: int
    boundary_edge_count: int
    nonmanifold_edge_count: int
    self_intersections: int | None = None
    self_intersections_available: bool = False


@dataclass(slots=True)
class ServiceMeshHealth:
    is_closed: bool
    self_intersections: int
    self_intersection_faces: list[int]
    holes_count: int
    degenerate_faces: int
    health_score: int


@dataclass(slots=True)
class RepairReport:
    input_vertex_count: int
    input_face_count: int
    output_vertex_count: int
    output_face_count: int
    merged_vertices: int = 0
    removed_degenerate_faces: int = 0
    removed_unreferenced_vertices: int = 0


@dataclass(slots=True)
class MeshHealerIssue:
    issue_id: str
    label: str
    count: int
    severity: Literal["info", "warning", "error"]
    rust_repair_available: bool
    repair_command: str | None = None


@dataclass(slots=True)
class MeshHealerReport:
    input_vertex_count: int
    input_face_count: int
    holes_count: int
    boundary_edge_count: int
    nonmanifold_edge_count: int
    self_intersections: int | None
    self_intersections_available: bool
    total_issue_count: int
    issue_category_count: int
    fixable_issue_count: int
    auto_repair_ready: bool
    issues: list[MeshHealerIssue]


@dataclass(slots=True)
class ComponentPruneReport:
    input_component_count: int
    output_component_count: int
    removed_component_count: int
    input_face_count: int
    output_face_count: int
    removed_face_count: int
    input_vertex_count: int
    output_vertex_count: int
    removed_vertex_count: int
    retained_face_count: int
    min_area_mm2: float


@dataclass(slots=True)
class HoleFillPlanEntry:
    hole_index: int
    representative_edge: tuple[int, int]
    boundary_vertex_indices: list[int]
    boundary_edge_count: int
    planned_triangles: int
    skipped: bool
    skip_reason: str | None


@dataclass(slots=True)
class HoleFillPlanDiagnostics:
    input_holes: int
    planned_holes: int
    skipped_holes: int
    total_boundary_edges: int
    total_planned_triangles: int
    max_edges: int | None
    plans: list[HoleFillPlanEntry]


@dataclass(slots=True)
class RepeatedHoleBoundaryVertexEntry:
    vertex_index: int
    hole_indices: list[int]
    occurrences: int


@dataclass(slots=True)
class RepeatedHoleBoundaryVerticesDiagnostics:
    input_holes: int
    repeated_vertex_count: int
    vertices: list[RepeatedHoleBoundaryVertexEntry]


@dataclass(slots=True)
class HoleComplicatingFaceEntry:
    repeated_vertex_index: int
    face_index: int


@dataclass(slots=True)
class HoleComplicatingFacesDiagnostics:
    input_repeated_vertex_count: int
    complicating_face_count: int
    faces: list[HoleComplicatingFaceEntry]


@dataclass(slots=True)
class RemoveHoleComplicatingFacesReport:
    input_face_count: int
    output_face_count: int
    removed_face_count: int
    input_repeated_vertex_count: int
    output_repeated_vertex_count: int


@dataclass(slots=True)
class ShortEdgeEntry:
    edge: tuple[int, int]
    length_mm: float


@dataclass(slots=True)
class ShortEdgeDiagnostics:
    critical_length_mm: float
    edge_count: int
    short_edge_count: int
    min_short_edge_length_mm: float | None
    max_short_edge_length_mm: float | None
    edges: list[ShortEdgeEntry]


@dataclass(slots=True)
class DegenerateFaceEntry:
    face_index: int
    face: tuple[int, int, int]
    aspect_ratio: float


@dataclass(slots=True)
class DegenerateFaceDiagnostics:
    critical_aspect_ratio: float
    face_count: int
    degenerate_face_count: int
    min_degenerate_aspect_ratio: float | None
    max_degenerate_aspect_ratio: float | None
    faces: list[DegenerateFaceEntry]


@dataclass(slots=True)
class MultipleEdgeEntry:
    vertex_pair: tuple[int, int]
    topology_edge_count: int
    face_edge_occurrences: int
    forward_occurrences: int
    reverse_occurrences: int


@dataclass(slots=True)
class MultipleEdgeDiagnostics:
    edge_count: int
    multiple_edge_count: int
    edges: list[MultipleEdgeEntry]


@dataclass(slots=True)
class MultipleEdgeRepairReport:
    input_edge_count: int
    output_edge_count: int
    input_multiple_edge_count: int
    output_multiple_edge_count: int
    split_edge_count: int
    split_face_count: int
    added_vertex_count: int
    input_face_count: int
    output_face_count: int


@dataclass(slots=True)
class NonManifoldEdgeRepairReport:
    input_nonmanifold_edge_count: int
    output_nonmanifold_edge_count: int
    removed_face_count: int
    input_vertex_count: int
    output_vertex_count: int
    input_face_count: int
    output_face_count: int


@dataclass(slots=True)
class DuplicateNonManifoldVerticesReport:
    input_nonmanifold_vertex_count: int
    output_nonmanifold_vertex_count: int
    duplicated_vertex_count: int
    input_vertex_count: int
    output_vertex_count: int
    input_face_count: int
    output_face_count: int


@dataclass(slots=True)
class DuplicateMultiHoleVerticesReport:
    input_multi_hole_vertex_count: int
    output_multi_hole_vertex_count: int
    duplicated_vertex_count: int
    input_vertex_count: int
    output_vertex_count: int
    input_face_count: int
    output_face_count: int


@dataclass(slots=True)
class NotSmoothFaceEntry:
    face_index: int
    face: tuple[int, int, int]
    angle_delta_radians: float


@dataclass(slots=True)
class NotSmoothFaceDiagnostics:
    min_angle_radians: float
    face_count: int
    not_smooth_face_count: int
    faces: list[NotSmoothFaceEntry]


@dataclass(slots=True)
class CreaseEdgeEntry:
    edge: tuple[int, int]
    face_indices: tuple[int, int]
    dihedral_cosine: float


@dataclass(slots=True)
class CreaseEdgeDiagnostics:
    angle_from_planar_radians: float
    min_component_length_mm: float | None
    min_branch_length_mm: float | None
    edge_count: int
    raw_crease_edge_count: int
    crease_edge_count: int
    edges: list[CreaseEdgeEntry]


@dataclass(slots=True)
class CreaseRepairPlanRegion:
    crease_edge: tuple[int, int]
    selected_origin_vertex: int
    selected_face_indices: list[int]


@dataclass(slots=True)
class CreaseRepairPlanDiagnostics:
    angle_from_planar_radians: float
    critical_tri_aspect_ratio: float
    crease_edge_count: int
    planned_region_count: int
    planned_face_count: int
    regions: list[CreaseRepairPlanRegion]


@dataclass(slots=True)
class FixMeshCreasesReport:
    input_face_count: int
    output_face_count: int
    input_crease_edge_count: int
    output_crease_edge_count: int
    repaired_region_count: int
    removed_face_count: int
    added_face_count: int
    filled_hole_count: int
    skipped_hole_count: int
    iteration_count: int


@dataclass(slots=True)
class FixSelfIntersectionsRelaxReport:
    input_vertex_count: int
    input_face_count: int
    output_vertex_count: int
    output_face_count: int
    input_self_intersections: int
    output_self_intersections: int
    relaxed_face_count: int
    moved_vertex_count: int
    relax_iterations: int
    max_expand: int
    force: float
    method: str
    subdivide_edge_len_disabled: bool
    topology_changed: bool


@dataclass(slots=True)
class HoleFillReport:
    input_holes: int
    filled_holes: int
    added_vertices: int
    added_faces: int
    new_face_indices: list[int] = field(default_factory=list)
    skipped_holes: int = 0


@dataclass(slots=True)
class VoxelRebuildReport:
    input_vertex_count: int
    input_face_count: int
    output_vertex_count: int
    output_face_count: int
    input_boundary_edge_count: int
    output_boundary_edge_count: int
    input_nonmanifold_edge_count: int
    output_nonmanifold_edge_count: int
    input_self_intersections: int | None
    output_self_intersections: int | None
    voxel_size_mm: float
    offset_mm: float
    extractor: str
    refine: bool


@dataclass(slots=True)
class TunnelDiagnostics:
    vertex_count: int
    face_count: int
    edge_count: int
    connected_component_count: int
    boundary_edge_count: int
    nonmanifold_edge_count: int
    euler_characteristic: int
    genus: int | None
    tunnel_count: int
    closed: bool


@dataclass(slots=True)
class TunnelEliminationReport:
    input_face_count: int
    detected_tunnel_face_count: int
    removed_face_count: int
    filled_holes: int
    added_faces: int
    output_face_count: int
    output_boundary_edge_count: int
    output_tunnel_count: int
    tunnel_face_indices: list[int]


@dataclass(slots=True)
class SDFGrid:
    origin: tuple[float, float, float]
    voxel_size_mm: float
    shape: tuple[int, int, int]
    values: np.ndarray

    def __post_init__(self) -> None:
        shape = tuple(int(value) for value in self.shape)
        if len(shape) != 3 or any(value <= 0 for value in shape):
            raise ValueError("shape must contain three positive values")
        values = np.asarray(self.values, dtype=np.float32)
        if values.size != shape[0] * shape[1] * shape[2]:
            raise ValueError("values size must match shape")
        self.shape = shape
        self.origin = tuple(float(value) for value in self.origin)
        self.voxel_size_mm = float(self.voxel_size_mm)
        self.values = np.ascontiguousarray(values.reshape(shape))

    def points(self) -> np.ndarray:
        from geometry_sdk.accelerators import _rust_sdf

        return _rust_sdf.sdf_grid_points(self)

    def point_to_grid(self, points: np.ndarray) -> np.ndarray:
        from geometry_sdk.accelerators import _rust_sdf

        return _rust_sdf.sdf_points_to_grid(self, points)


@dataclass(slots=True)
class VoxelVolume:
    dimensions: tuple[int, int, int]
    voxel_size: tuple[float, float, float]
    grid_level_set: bool
    scalar_type: str
    values: np.ndarray
    min_value: float
    max_value: float
    metadata: dict[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        dimensions = tuple(int(value) for value in self.dimensions)
        if len(dimensions) != 3 or any(value <= 0 for value in dimensions):
            raise ValueError("dimensions must contain three positive values")
        voxel_size = tuple(float(value) for value in self.voxel_size)
        if len(voxel_size) != 3 or any(not np.isfinite(value) or value <= 0.0 for value in voxel_size):
            raise ValueError("voxel_size must contain three positive finite values")
        values = np.asarray(self.values, dtype=np.float32)
        if values.size != dimensions[0] * dimensions[1] * dimensions[2]:
            raise ValueError("values size must match dimensions")
        self.dimensions = dimensions
        self.voxel_size = voxel_size
        self.values = np.ascontiguousarray(values.reshape(dimensions))
        self.min_value = float(self.min_value)
        self.max_value = float(self.max_value)


@dataclass(slots=True)
class VoxelPathResult:
    voxel_indices: list[int]
    coordinates: list[tuple[int, int, int]]
    total_metric: float


@dataclass(slots=True)
class VoxelSliceResult:
    width: int
    height: int
    values: np.ndarray
    normalized_values: np.ndarray
    coordinates: list[tuple[int, int, int]]


@dataclass(slots=True)
class VoxelLineGraphResult:
    axis: int
    positions: list[int]
    voxel_indices: list[int]
    coordinates: list[tuple[int, int, int]]
    values: np.ndarray


@dataclass(slots=True)
class VoxelActiveBoxResult:
    min_corner: tuple[int, int, int]
    dimensions: tuple[int, int, int]
    source_indices: list[int]
    coordinates: list[tuple[int, int, int]]
    values: np.ndarray


@dataclass(slots=True)
class VoxelVolumeRenderDataResult:
    dimensions: tuple[int, int, int]
    voxel_size: tuple[float, float, float]
    source_indices: list[int]
    coordinates: list[tuple[int, int, int]]
    values: np.ndarray
    min_value: float
    max_value: float
    metadata: dict[str, Any] = field(default_factory=dict)


@dataclass(slots=True)
class VoxelVolumeRenderLutResult:
    lut_type: str
    alpha_type: str
    alpha_limit: int
    colors_rgba: list[tuple[int, int, int, int]]
    metadata: dict[str, Any] = field(default_factory=dict)


@dataclass(slots=True)
class VoxelVolumeRenderRayResult:
    color_rgba: np.ndarray
    first_opaque_world: tuple[float, float, float] | None
    visited_indices: list[int]
    accepted_indices: list[int]
    metadata: dict[str, Any] = field(default_factory=dict)


@dataclass(slots=True)
class VoxelSegmentationResult:
    min_corner: tuple[int, int, int]
    dimensions: tuple[int, int, int]
    source_indices: list[int]
    part_indices: list[int]
    selected_coordinates: list[tuple[int, int, int]]
    selected_values: np.ndarray


@dataclass(slots=True)
class ThicknessSummary:
    min_mm: float | None
    avg_mm: float | None
    max_mm: float | None
    valid_vertex_count: int
    violation_count: int


@dataclass(slots=True)
class VersionCompareSummary:
    volume_delta_mm3: float
    bbox_delta_mm: tuple[float, float, float]
    min_signed_distance_mm: float | None = None
    max_signed_distance_mm: float | None = None
    mean_signed_distance_mm: float | None = None
    weight_delta_g: float = 0.0


@dataclass(slots=True)
class SectionContourSegment:
    start: tuple[float, float, float]
    end: tuple[float, float, float]
    selected_region_hit: bool


@dataclass(slots=True)
class SectionContourPayload:
    section_constant: float
    plane_axis: tuple[float, float, float]
    plane_u_axis: tuple[float, float, float]
    plane_v_axis: tuple[float, float, float]
    plane_origin: tuple[float, float, float]
    contour_count: int
    segment_count: int
    selected_region_segment_count: int
    perimeter_mm: float | None
    width_mm: float | None
    depth_mm: float | None
    projected_bounds_min: tuple[float, float] | None
    projected_bounds_max: tuple[float, float] | None
    bounds_min: tuple[float, float, float] | None
    bounds_max: tuple[float, float, float] | None
    segments: list[SectionContourSegment]


@dataclass(slots=True)
class MaterialWeightEntry:
    volume_mm3: float
    weight_g: float


@dataclass(slots=True)
class DrainHolePlan:
    center_mm: tuple[float, float, float]
    direction: tuple[float, float, float]
    radius_mm: float
    length_mm: float


@dataclass(slots=True)
class AdaptiveHollowReport:
    achieved_weight_g: float
    wall_thickness_mm: float | None
    iterations: int
    warning: str | None
    original_weight_g: float
    target_weight_g: float


@dataclass(slots=True)
class ManufacturabilityReport:
    health: MeshHealth
    stats: MeshStats
    ring_measurement: "RingMeasurement"
    thickness: ThicknessSummary
    regions: list["RegionEntry"]
    material_weights: dict[str, MaterialWeightEntry]
    recommendations: list[str]
    export_ready: bool
    health_score: int


@dataclass(slots=True)
class RingMeasurement:
    ring_axis: tuple[float, float, float]
    ring_axis_confidence: float
    estimated_ring_size_us: float | None
    inner_diameter_mm: float | None
    band_width_min_mm: float | None
    band_width_max_mm: float | None
    head_height_mm: float | None
    bbox_mm: tuple[float, float, float]
    needs_axis_confirmation: bool


@dataclass(slots=True)
class RegionEntry:
    region_id: str
    label: str
    vertex_indices: np.ndarray
    coverage_pct: float
    protected_by_default: bool
    allowed_operations: list[str]
    min_thickness_mm: float | None = None
    avg_thickness_mm: float | None = None
    violation_count: int = 0
    centroid_mm: tuple[float, float, float] | None = None
