"""Pydantic schemas for the production-grade API."""

from __future__ import annotations

from datetime import datetime
from enum import Enum
from typing import Any, Literal

from pydantic import BaseModel, Field, model_validator


class MaterialType(str, Enum):
    GOLD_24K = "gold_24k"
    GOLD_22K = "gold_22k"
    GOLD_18K = "gold_18k"
    GOLD_14K = "gold_14k"
    GOLD_10K = "gold_10k"
    SILVER_925 = "silver_925"
    PLATINUM = "platinum"


class OperationType(str, Enum):
    INGEST = "ingest"
    BRANCH = "branch"
    REPAIR = "repair"
    RESIZE = "resize"
    HOLLOW = "hollow"
    THICKEN = "thicken"
    SCOOP = "scoop"
    SMOOTH = "smooth"
    DECIMATE = "decimate"
    SUBDIVIDE = "subdivide"
    COMPARE = "compare"
    MAKE_MANUFACTURABLE = "make_manufacturable"
    INTERACTIVE_COMMIT = "interactive_commit"
    INTERACTIVE_BRUSH_REPLAY = "interactive_brush_replay"
    SELECTION_TO_OBJECT = "selection_to_object"
    MESH_CUT_MEASURE = "mesh_cut_measure"


class ArtifactSummary(BaseModel):
    id: str
    artifact_type: str
    mime_type: str
    storage_key: str
    size_bytes: int
    metadata_json: dict[str, Any] = Field(default_factory=dict)


class ModelVersionSummary(BaseModel):
    id: str
    model_id: str
    parent_version_id: str | None = None
    operation_type: str
    operation_label: str
    status: str
    created_at: datetime


class ModelSummary(BaseModel):
    id: str
    source_filename: str
    source_type: str
    created_at: datetime
    latest_version_id: str | None = None


class MeshHealthSnapshot(BaseModel):
    is_closed: bool
    holes_count: int
    self_intersections: int
    disconnected_shells: int
    health_score: int


class DimensionsSnapshot(BaseModel):
    unit_system: Literal["mm"] = "mm"
    ring_axis: tuple[float, float, float] | None = None
    ring_axis_confidence: float
    estimated_ring_size_us: float | None = None
    inner_diameter_mm: float | None = None
    band_width_min_mm: float | None = None
    band_width_max_mm: float | None = None
    head_height_mm: float | None = None
    bbox_mm: tuple[float, float, float]
    needs_axis_confirmation: bool = False


class MaterialWeightEntry(BaseModel):
    volume_mm3: float
    weight_g: float


class ThicknessSnapshot(BaseModel):
    min_mm: float | None
    avg_mm: float | None
    max_mm: float | None
    violation_count: int
    threshold_mm: float
    scalar_field_artifact_id: str | None = None


class RegionManifestEntry(BaseModel):
    region_id: str
    label: str
    vertex_count: int
    coverage_pct: float
    min_thickness_mm: float | None = None
    avg_thickness_mm: float | None = None
    violation_count: int = 0
    protected_by_default: bool = False
    allowed_operations: list[str] = Field(default_factory=list)
    centroid_mm: tuple[float, float, float] | None = None


class ManufacturabilitySnapshot(BaseModel):
    version_id: str
    mesh_health: MeshHealthSnapshot
    dimensions: DimensionsSnapshot
    material_weight: dict[MaterialType, MaterialWeightEntry]
    thickness: ThicknessSnapshot
    regions: list[RegionManifestEntry] = Field(default_factory=list)
    recommendations: list[str]
    export_ready: bool


class TextureArtifactManifest(BaseModel):
    texture_index: int
    artifact_url: str
    metadata: dict[str, Any] = Field(default_factory=dict)


class ViewerManifest(BaseModel):
    version_id: str
    preview_low_url: str | None = None
    preview_high_url: str | None = None
    normalized_mesh_url: str | None = None
    meshlib_scene_object_url: str | None = None
    meshlib_scene_object_metadata: dict[str, Any] = Field(default_factory=dict)
    meshlib_scene_mru_url: str | None = None
    meshlib_scene_mru_metadata: dict[str, Any] = Field(default_factory=dict)
    texture_artifact_url: str | None = None
    texture_metadata: dict[str, Any] = Field(default_factory=dict)
    texture_artifacts: list[TextureArtifactManifest] = Field(default_factory=list)
    texture_per_face: list[int] = Field(default_factory=list)
    thickness_artifact_url: str | None = None
    region_artifact_url: str | None = None
    bounding_box: tuple[float, float, float]
    default_material: MaterialType = MaterialType.GOLD_18K
    available_overlays: list[str] = Field(default_factory=list)
    region_manifest: list[RegionManifestEntry] = Field(default_factory=list)
    measurements_summary: dict[str, Any] = Field(default_factory=dict)
    can_edit: bool = True
    needs_axis_confirmation: bool = False


class InteractiveSelectionPayload(BaseModel):
    mode: Literal["brush", "lasso", "rect", "pick", "faces", "vertices", "regions"] = "brush"
    vertex_ids: list[int] = Field(default_factory=list)
    face_ids: list[int] = Field(default_factory=list)
    region_ids: list[str] = Field(default_factory=list)
    brush_points_world: list[tuple[float, float, float]] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)


class InteractiveCommitRequest(BaseModel):
    tool_id: Literal[
        "select_mark_region",
        "thicken_brush",
        "scoop_brush",
        "smooth_brush",
        "measure_inspect",
        "meshlib_workbench_export",
    ] = "select_mark_region"
    operation_label: str = Field(default="Interactive Edit Commit", min_length=1, max_length=255)
    selection: InteractiveSelectionPayload | None = None
    brush_radius_mm: float | None = Field(default=None, gt=0)
    falloff_mm: float | None = Field(default=None, gt=0)
    target_thickness_mm: float | None = Field(default=None, gt=0)
    depth_mm: float | None = Field(default=None, gt=0)
    min_thickness_mm: float | None = Field(default=None, ge=0.2)
    iterations: int | None = Field(default=None, ge=1, le=100)
    strength: float | None = Field(default=None, ge=0.0, le=1.0)
    preserve_detail: bool = True
    metadata: dict[str, Any] = Field(default_factory=dict)


class SelectionCommitRequest(BaseModel):
    tool_id: Literal["select_mark_region"] = "select_mark_region"
    operation_label: str = Field(default="Select / Mark Region", min_length=1, max_length=255)
    selection: InteractiveSelectionPayload
    label: str | None = Field(default=None, max_length=120)
    create_object: bool = False
    metadata: dict[str, Any] = Field(default_factory=dict)


class SelectionCommitResponse(BaseModel):
    version_id: str
    artifact_id: str
    artifact_url: str
    artifact_type: str = "meshlib_selection_json"
    selection_counts: dict[str, int] = Field(default_factory=dict)
    resolved_counts: dict[str, int] = Field(default_factory=dict)
    selected_object_version_id: str | None = None
    selected_object_artifact_id: str | None = None
    selected_object_artifact_url: str | None = None
    selected_object_artifact_type: str | None = None
    selected_object_counts: dict[str, int] | None = None


class BrushReplayStroke(BaseModel):
    tool_id: Literal["thicken_brush", "scoop_brush", "smooth_brush"]
    selection: InteractiveSelectionPayload
    amount_mm: float = Field(default=0.15, ge=0.0)
    falloff_mm: float = Field(default=1.5, gt=0)
    iterations: int = Field(default=1, ge=1, le=100)
    strength: float = Field(default=0.5, ge=0.0, le=1.0)
    metadata: dict[str, Any] = Field(default_factory=dict)


class BrushReplayRequest(BaseModel):
    operation_label: str = Field(default="MeshLib Brush Replay", min_length=1, max_length=255)
    strokes: list[BrushReplayStroke] = Field(..., min_length=1, max_length=256)
    metadata: dict[str, Any] = Field(default_factory=dict)


class MeasureInspectPair(BaseModel):
    start: tuple[float, float, float]
    end: tuple[float, float, float]
    label: str | None = Field(default=None, max_length=120)
    metric: Literal["euclidean", "geodesic"] = "euclidean"
    start_vertex: int | None = Field(default=None, ge=0)
    end_vertex: int | None = Field(default=None, ge=0)
    start_face_index: int | None = Field(default=None, ge=0)
    start_barycentric: tuple[float, float, float] | None = None
    end_face_index: int | None = Field(default=None, ge=0)
    end_barycentric: tuple[float, float, float] | None = None
    control_vertices: list[int] = Field(default_factory=list, max_length=128)
    close_path: bool = False
    geodesic_max_path_len_mm: float | None = Field(default=None, gt=0)
    include_refined_surface_path: bool = False


class MeasureInspectSurfaceDistanceRequest(BaseModel):
    seed: tuple[float, float, float] | None = None
    seed_vertex: int | None = Field(default=None, ge=0)
    seed_vertices: list[int] = Field(default_factory=list, max_length=64)
    seed_edges: list[tuple[int, int]] = Field(default_factory=list, max_length=128)
    seed_face_ids: list[int] = Field(default_factory=list, max_length=512)
    max_distance_mm: float | None = Field(default=None, gt=0)
    iso_value_mm: float | None = Field(default=None, ge=0)
    include_distances: bool = True
    include_iso_segments: bool = True
    include_extreme_edges: bool = False


class MeasureInspectFeaturePrimitive(BaseModel):
    feature_id: str = Field(..., min_length=1, max_length=128)
    kind: Literal["point", "sphere", "line", "plane", "circle", "cylinder", "cone"]
    center: tuple[float, float, float] = (0.0, 0.0, 0.0)
    direction: tuple[float, float, float] | None = None
    normal: tuple[float, float, float] | None = None
    radius_mm: float = Field(default=0.0, ge=0)
    length_mm: float = Field(default=0.0, ge=0)


class MeasureInspectFeaturePair(BaseModel):
    first_feature_id: str = Field(..., min_length=1, max_length=128)
    second_feature_id: str = Field(..., min_length=1, max_length=128)
    label: str | None = None


class MeasureInspectFeatureRefineRequest(BaseModel):
    feature_id: str = Field(..., min_length=1, max_length=128)
    distance_limit_mm: float = Field(default=0.1, ge=0)
    normal_tolerance_degrees: float = Field(default=30.0, ge=0, le=180)
    max_iterations: int = Field(default=10, ge=1, le=64)
    label: str | None = None


class MeasureInspectRequest(BaseModel):
    points: list[tuple[float, float, float]] = Field(default_factory=list, max_length=64)
    point_pairs: list[MeasureInspectPair] = Field(default_factory=list, max_length=32)
    features: list[MeasureInspectFeaturePrimitive] = Field(default_factory=list, max_length=64)
    feature_pairs: list[MeasureInspectFeaturePair] = Field(default_factory=list, max_length=64)
    feature_refinements: list[MeasureInspectFeatureRefineRequest] = Field(default_factory=list, max_length=64)
    include_feature_objects: bool = True
    feature_object_infinite_extent_mm: float = Field(default=1000.0, gt=0)
    surface_distance: MeasureInspectSurfaceDistanceRequest | None = None
    include_local_thickness: bool = True


class MeasureInspectPointResult(BaseModel):
    query_point: tuple[float, float, float]
    closest_point: tuple[float, float, float]
    face_index: int
    distance_to_surface_mm: float
    local_thickness_mm: float | None = None


class MeasureInspectPairResult(BaseModel):
    start: tuple[float, float, float]
    end: tuple[float, float, float]
    distance_mm: float
    midpoint: tuple[float, float, float]
    label: str | None = None
    metric: Literal["euclidean", "geodesic"] = "euclidean"
    control_vertex_indices: list[int] = Field(default_factory=list)
    control_vertex_offsets: list[int] = Field(default_factory=list)
    path_vertex_indices: list[int] = Field(default_factory=list)
    path_points: list[tuple[float, float, float]] = Field(default_factory=list)
    path_point_normals: list[tuple[float, float, float]] = Field(default_factory=list)
    edge_lengths_mm: list[float] = Field(default_factory=list)
    leg_lengths_mm: list[float] = Field(default_factory=list)
    leg_vertex_offsets: list[int] = Field(default_factory=list)
    line_segments: int = 0
    closed_path: bool = False
    path_object_lines: dict[str, Any] | None = None
    path_object_points: dict[str, Any] | None = None
    cut_contours: dict[str, Any] | None = None
    surface_path_refinement: dict[str, Any] | None = None
    meshlib_reference: str | None = None


class MeasureInspectSurfaceDistanceResult(BaseModel):
    seed: tuple[float, float, float] | None = None
    seed_vertex: int
    seed_vertices: list[int] = Field(default_factory=list)
    seed_edges: list[tuple[int, int]] = Field(default_factory=list)
    seed_face_ids: list[int] = Field(default_factory=list)
    seed_face_boundary_edges: list[tuple[int, int]] = Field(default_factory=list)
    distances_mm: list[float | None] = Field(default_factory=list)
    predecessor_vertices: list[int | None] = Field(default_factory=list)
    reachable_vertex_count: int
    max_distance_mm: float
    iso_value_mm: float | None = None
    selected_vertex_indices: list[int] = Field(default_factory=list)
    selected_face_indices: list[int] = Field(default_factory=list)
    crossing_face_indices: list[int] = Field(default_factory=list)
    boundary_edges: list[tuple[int, int]] = Field(default_factory=list)
    iso_segments: list[tuple[tuple[float, float, float], tuple[float, float, float]]] = Field(default_factory=list)
    ridge_edges: list[tuple[int, int]] = Field(default_factory=list)
    gorge_edges: list[tuple[int, int]] = Field(default_factory=list)
    clipped_vertices: list[tuple[float, float, float]] = Field(default_factory=list)
    clipped_faces: list[tuple[int, int, int]] = Field(default_factory=list)
    clipped_source_face_indices: list[int] = Field(default_factory=list)
    clipped_source_vertex_indices: list[int | None] = Field(default_factory=list)
    meshlib_reference: str = "MR::computeClosestSurfacePathTargets"


class MeasureInspectFeatureDistanceResult(BaseModel):
    status: Literal["ok", "bad_feature_pair", "bad_relative_location", "not_implemented", "not_finite"]
    distance_mm: float | None = None
    closest_point_a: tuple[float, float, float] | None = None
    closest_point_b: tuple[float, float, float] | None = None


class MeasureInspectFeatureAngleResult(BaseModel):
    status: Literal["ok", "bad_feature_pair", "bad_relative_location", "not_implemented", "not_finite"]
    angle_radians: float | None = None
    angle_degrees: float | None = None
    point_a: tuple[float, float, float] | None = None
    point_b: tuple[float, float, float] | None = None
    direction_a: tuple[float, float, float] | None = None
    direction_b: tuple[float, float, float] | None = None
    is_surface_normal_a: bool = False
    is_surface_normal_b: bool = False


class MeasureInspectFeatureIntersectionResult(BaseModel):
    kind: Literal["point", "sphere", "line", "plane", "circle", "cylinder", "cone"]
    center: tuple[float, float, float]
    direction: tuple[float, float, float] | None = None
    radius_mm: float | None = None
    length_mm: float | None = None
    start_point: tuple[float, float, float] | None = None
    end_point: tuple[float, float, float] | None = None
    meshlib_primitive: str


class MeasureInspectFeaturePairResult(BaseModel):
    first_feature_id: str
    second_feature_id: str
    first_kind: Literal["point", "sphere", "line", "plane", "circle", "cylinder", "cone"]
    second_kind: Literal["point", "sphere", "line", "plane", "circle", "cylinder", "cone"]
    label: str | None = None
    distance: MeasureInspectFeatureDistanceResult
    center_distance: MeasureInspectFeatureDistanceResult
    angle: MeasureInspectFeatureAngleResult
    intersections: list[MeasureInspectFeatureIntersectionResult] = Field(default_factory=list)
    meshlib_reference: str = "MR::Features::MeasureResult"


class MeasureInspectFeatureObjectPropertyResult(BaseModel):
    name: str
    kind: Literal["position", "linear_dimension", "direction", "angle", "other"]
    scalar_value: float | None = None
    vector_value: tuple[float, float, float] | None = None


class MeasureInspectFeatureObjectResult(BaseModel):
    feature_id: str
    source_kind: Literal["point", "sphere", "line", "plane", "circle", "cylinder", "cone"]
    object_type: Literal["PointObject", "SphereObject", "LineObject", "PlaneObject", "CircleObject", "CylinderObject", "ConeObject"]
    class_name: str = "Feature"
    class_name_plural: str = "Features"
    shared_properties: list[MeasureInspectFeatureObjectPropertyResult] = Field(default_factory=list)
    meshlib_reference: str = "MR::Features::primitiveToObject"


class MeasureInspectFeatureRefinementResult(BaseModel):
    feature_id: str
    kind: Literal["point", "sphere", "line", "plane", "circle", "cylinder", "cone"]
    label: str | None = None
    center: tuple[float, float, float]
    direction: tuple[float, float, float] | None = None
    radius_mm: float = 0.0
    length_mm: float = 0.0
    selected_vertex_indices: list[int] = Field(default_factory=list)
    selected_count: int
    iterations: int
    converged: bool
    feature_object: MeasureInspectFeatureObjectResult | None = None
    meshlib_reference: str = "MR::refineFeatureObject"


class MeasureInspectResponse(BaseModel):
    version_id: str
    points: list[MeasureInspectPointResult] = Field(default_factory=list)
    point_pairs: list[MeasureInspectPairResult] = Field(default_factory=list)
    feature_pairs: list[MeasureInspectFeaturePairResult] = Field(default_factory=list)
    feature_objects: list[MeasureInspectFeatureObjectResult] = Field(default_factory=list)
    feature_refinements: list[MeasureInspectFeatureRefinementResult] = Field(default_factory=list)
    surface_distance: MeasureInspectSurfaceDistanceResult | None = None


class MeshCutMeasureTopologyRequest(BaseModel):
    control_vertices: list[int] = Field(..., min_length=2, max_length=128)
    close_path: bool = False
    max_path_len_mm: float | None = Field(default=None, gt=0)
    operation_label: str | None = None


class MeshCutMeasureTopologyResponse(BaseModel):
    version: ModelVersionSummary
    source_version_id: str
    artifact_id: str
    artifact_url: str
    control_vertices: list[int]
    closed_path: bool
    length_mm: float
    output_vertex_count: int
    output_face_count: int
    duplicate_vertex_map: list[list[int]] = Field(default_factory=list)
    source_path_vertex_indices: list[int] = Field(default_factory=list)
    result_cut_vertex_indices: list[list[int]] = Field(default_factory=list)
    cut_edge_pairs: list[list[int]] = Field(default_factory=list)
    result_cut_edge_pairs: list[list[int]] = Field(default_factory=list)
    bad_face_indices: list[int] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)


class GcodeParsePathsRequest(BaseModel):
    source: str = Field(..., min_length=1)
    machine_settings: dict[str, Any] | None = None


class GcodeLoadSourceRequest(BaseModel):
    file_name: str = "program.gcode"
    source: str = Field(..., min_length=1)


class GcodeSourceResponse(BaseModel):
    version_id: str
    file_name: str
    frame_count: int
    source_frames: list[str] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)


class GcodeWriteSourceRequest(BaseModel):
    file_name: str = "program.gcode"
    source_frames: list[str] = Field(default_factory=list)


class GcodeParseFilePathsRequest(GcodeLoadSourceRequest):
    machine_settings: dict[str, Any] | None = None


class GcodeParsePathsResponse(BaseModel):
    version_id: str
    frame_count: int
    command_count: int
    segment_count: int
    max_feedrate: float
    unit: str = "mm"
    segments: list[list[list[float]]] = Field(default_factory=list)
    tool_directions: list[list[list[float]]] = Field(default_factory=list)
    source_frame_indices: list[int] = Field(default_factory=list)
    idle: list[bool] = Field(default_factory=list)
    feedrates: list[float] = Field(default_factory=list)
    warnings: list[str] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)


class PointCloudIcpRequest(BaseModel):
    floating_points: list[tuple[float, float, float]] = Field(..., min_length=1)
    reference_points: list[tuple[float, float, float]] = Field(..., min_length=1)
    method: Literal["point_to_point", "point_to_plane"] = "point_to_point"
    mode: Literal["rigid", "translation"] = "rigid"
    max_iterations: int = Field(default=20, ge=1, le=500)
    tolerance: float = Field(default=1e-8, gt=0)
    reference_normals: list[tuple[float, float, float]] | None = None
    floating_normals: list[tuple[float, float, float]] | None = None
    max_pair_distance: float | None = Field(default=None, gt=0)
    cos_threshold: float | None = Field(default=None, ge=-1.0, le=1.0)
    far_dist_factor: float | None = Field(default=None, gt=0)
    mutual_closest: bool = False


class PointCloudIcpResponse(BaseModel):
    version_id: str
    method: Literal["point_to_point", "point_to_plane"]
    mode: Literal["rigid", "translation"]
    rotation: list[list[float]]
    translation: tuple[float, float, float]
    transform: list[list[float]]
    iterations: int
    mean_square_distance: float
    active_pair_count: int
    metadata: dict[str, Any] = Field(default_factory=dict)


class PointCloudTriangulationRequest(BaseModel):
    points: list[tuple[float, float, float]] = Field(..., min_length=3)
    stage: Literal["candidate", "cleaned", "topology", "filled"] = "filled"
    radius: float = Field(default=0.0, ge=0)
    num_neighbors: int = Field(default=16, ge=0, le=4096)
    boundary_angle: float = Field(default=2.827433388230814, gt=0)
    max_removes: int = Field(default=2_147_483_647, ge=0, le=2_147_483_647)
    crit_angle: float = Field(default=6.283185307179586, gt=0)
    crit_hole_length: float = -1.0
    normals: list[tuple[float, float, float]] | None = None
    untrusted_indices: list[int] | None = None

    @model_validator(mode="after")
    def ensure_single_neighborhood_control(self) -> "PointCloudTriangulationRequest":
        if (self.radius > 0) == (self.num_neighbors > 0):
            raise ValueError("exactly one of radius or num_neighbors must be positive")
        return self


class PointCloudTriangulationResponse(BaseModel):
    version_id: str
    stage: Literal["candidate", "cleaned", "topology", "filled"]
    vertices: list[tuple[float, float, float]]
    faces: list[tuple[int, int, int]]
    vertex_count: int
    face_count: int
    metadata: dict[str, Any] = Field(default_factory=dict)


class PointCloudMultiwayIcpTransform(BaseModel):
    rotation: list[list[float]]
    translation: tuple[float, float, float]
    transform: list[list[float]]


class PointCloudMultiwayIcpRequest(BaseModel):
    objects: list[list[tuple[float, float, float]]] = Field(..., min_length=2)
    method: Literal["point_to_point", "point_to_plane", "combined"] = "point_to_point"
    grouping: Literal["independent", "all_object", "sequential_cascade", "aabb_cascade"] = "independent"
    mode: Literal["rigid", "translation"] = "rigid"
    max_iterations: int = Field(default=20, ge=1, le=500)
    tolerance: float = Field(default=1e-8, gt=0)
    fixed_object_index: int | None = Field(default=None, ge=0)
    max_group_size: int = Field(default=64, ge=2, le=4096)
    normals: list[list[tuple[float, float, float]]] | None = None

    @model_validator(mode="after")
    def ensure_multiway_inputs_are_complete(self) -> "PointCloudMultiwayIcpRequest":
        if any(len(points) == 0 for points in self.objects):
            raise ValueError("multiway ICP objects must not be empty")
        if self.fixed_object_index is not None and self.fixed_object_index >= len(self.objects):
            raise ValueError("fixed_object_index must reference an object")
        if self.method in {"point_to_plane", "combined"}:
            if self.normals is None:
                raise ValueError("normals are required for point_to_plane and combined multiway ICP")
            if len(self.normals) != len(self.objects):
                raise ValueError("normals must contain one normal cloud per object")
            for points, normals in zip(self.objects, self.normals, strict=True):
                if len(points) != len(normals):
                    raise ValueError("normal cloud sizes must match object point-cloud sizes")
        return self


class PointCloudMultiwayIcpResponse(BaseModel):
    version_id: str
    method: Literal["point_to_point", "point_to_plane", "combined"]
    grouping: Literal["independent", "all_object", "sequential_cascade", "aabb_cascade"]
    mode: Literal["rigid", "translation"]
    transforms: list[PointCloudMultiwayIcpTransform]
    iterations: int
    mean_square_distance: float
    active_pair_count: int
    fixed_object_index: int
    metadata: dict[str, Any] = Field(default_factory=dict)


class OffsetContoursRequest(BaseModel):
    contours: list[list[tuple[float, float, float]]] = Field(..., min_length=1)
    offset: float | None = None
    offsets: list[list[float]] | None = None
    min_angle_precision: float = Field(default=0.3490658503988659, gt=0)
    mode: Literal["offset", "shell"] = "offset"
    end_type: Literal["round", "cut"] = "round"
    corner_type: Literal["round", "sharp"] = "round"
    max_sharp_angle: float = Field(default=2.0943951023931953, gt=0)
    z_restore: Literal["default", "none", "constant", "custom"] = "default"
    z_value: float | None = None
    z_values: list[list[float]] | None = None
    relax_iterations: int = Field(default=1, ge=0, le=128)
    include_origins: bool = False


class OffsetContoursResponse(BaseModel):
    version_id: str
    contour_count: int
    point_count: int
    contours: list[list[tuple[float, float, float]]] = Field(default_factory=list)
    origins: list[Any] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)


class DistanceMapContoursRequest(BaseModel):
    contours: list[list[tuple[float, float]]] = Field(..., min_length=1)
    width: int = Field(..., ge=1, le=4096)
    height: int = Field(..., ge=1, le=4096)
    origin: tuple[float, float] = (0.0, 0.0)
    pixel_size: float | tuple[float, float] = Field(default=1.0)
    signed: bool = False


class DistanceMapFromMeshRequest(BaseModel):
    width: int = Field(..., ge=1, le=4096)
    height: int = Field(..., ge=1, le=4096)
    origin: tuple[float, float, float]
    x_range: tuple[float, float, float]
    y_range: tuple[float, float, float]
    direction: tuple[float, float, float]
    epsilon: float = Field(default=1e-8, gt=0)


class DistanceMapTiffImportRequest(BaseModel):
    file_name: str = "distance-map.tiff"
    contents_base64: str = Field(..., min_length=1)


class DistanceMapResponse(BaseModel):
    version_id: str
    width: int
    height: int
    origin: tuple[float, float]
    pixel_size: tuple[float, float]
    valid_count: int
    min_value: float
    max_value: float
    values: list[list[float]] = Field(default_factory=list)
    model_transform: list[float] | None = None
    unit: str = "mm"
    metadata: dict[str, Any] = Field(default_factory=dict)


class DistanceMapIsoLinesRequest(BaseModel):
    width: int = Field(..., ge=1, le=4096)
    height: int = Field(..., ge=1, le=4096)
    origin: tuple[float, float] = (0.0, 0.0)
    pixel_size: tuple[float, float] = (1.0, 1.0)
    values: list[list[float]] = Field(..., min_length=1)
    valid_count: int | None = Field(default=None, ge=0)
    min_value: float | None = None
    max_value: float | None = None
    model_transform: list[float] | None = None
    unit: str = "mm"
    iso_value: float = 0.0


class DistanceMapPayload(BaseModel):
    width: int = Field(..., ge=1, le=4096)
    height: int = Field(..., ge=1, le=4096)
    origin: tuple[float, float] = (0.0, 0.0)
    pixel_size: tuple[float, float] = (1.0, 1.0)
    values: list[list[float]] = Field(..., min_length=1)
    valid_count: int | None = Field(default=None, ge=0)
    min_value: float | None = None
    max_value: float | None = None
    model_transform: list[float] | None = None
    unit: str = "mm"


class DistanceMapTiffExportRequest(DistanceMapPayload):
    file_name: str = "distance-map.tiff"


class DistanceMapTiffExportResponse(BaseModel):
    version_id: str
    file_name: str
    byte_count: int
    contents_base64: str
    metadata: dict[str, Any] = Field(default_factory=dict)


class DistanceMapMergeRequest(BaseModel):
    left: DistanceMapPayload
    right: DistanceMapPayload
    mode: Literal["min", "max", "subtract"] = "min"


class DistanceMapContourBooleanRequest(BaseModel):
    contours_a: list[list[tuple[float, float]]] = Field(..., min_length=1)
    contours_b: list[list[tuple[float, float]]] = Field(..., min_length=1)
    mode: Literal["union", "intersection", "subtract"] = "union"
    width: int = Field(..., ge=1, le=4096)
    height: int = Field(..., ge=1, le=4096)
    origin: tuple[float, float] = (0.0, 0.0)
    pixel_size: tuple[float, float] = (1.0, 1.0)
    iso_value: float = 0.0


class IsoLineSegmentsResponse(BaseModel):
    version_id: str
    iso_value: float
    segment_count: int
    segments: list[tuple[tuple[float, float], tuple[float, float]]] = Field(default_factory=list)
    unit: str = "mm"
    metadata: dict[str, Any] = Field(default_factory=dict)


class ObjectLinesFromContoursRequest(BaseModel):
    contours: list[list[tuple[float, float, float]]] = Field(..., min_length=1)
    line_width: float = Field(default=1.0, ge=0)
    show_points: int = Field(default=0, ge=0)
    smooth_connections: int = Field(default=0, ge=0)


class ObjectLinesPtsLoadRequest(BaseModel):
    file_name: str = "object-lines.pts"
    source: str = Field(..., min_length=1)


class ObjectLinesSvgLoadRequest(BaseModel):
    file_name: str = "object-lines.svg"
    source: str = Field(..., min_length=1)


class ObjectLinesBinaryLoadRequest(BaseModel):
    file_name: str = "object-lines.mrlines"
    contents_base64: str = Field(..., min_length=1)


class ObjectLinesResponse(BaseModel):
    version_id: str
    point_count: int
    line_count: int
    line_width: float
    object_lines: dict[str, Any]
    metadata: dict[str, Any] = Field(default_factory=dict)


class ObjectLinesTextExportRequest(BaseModel):
    file_name: str = "object-lines.pts"
    object_lines: dict[str, Any]


class ObjectLinesTextExportResponse(BaseModel):
    version_id: str
    file_name: str
    source: str
    byte_count: int
    metadata: dict[str, Any] = Field(default_factory=dict)


class ObjectLinesBinaryExportRequest(BaseModel):
    file_name: str = "object-lines.mrlines"
    object_lines: dict[str, Any]


class ObjectLinesBinaryExportResponse(BaseModel):
    version_id: str
    file_name: str
    byte_count: int
    contents_base64: str
    metadata: dict[str, Any] = Field(default_factory=dict)


class ObjectLinesToContoursRequest(BaseModel):
    object_lines: dict[str, Any]


class ObjectLinesToContoursResponse(BaseModel):
    version_id: str
    contour_count: int
    point_count: int
    contours: list[list[tuple[float, float, float]]] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)


class MeshToVoxelsSdfRequest(BaseModel):
    voxel_size_mm: float = Field(default=1.0, gt=0)
    surface_offset_voxels: float = Field(default=3.0, gt=0)
    mode: Literal["signed", "unsigned"] = "signed"
    iso_value: float = 0.0
    extract_surface: bool = True


class MeshToVoxelsSdfResponse(BaseModel):
    version_id: str
    mode: Literal["signed", "unsigned"]
    voxel_size_mm: float
    surface_offset_voxels: float
    padding_mm: float
    iso_value: float
    origin: tuple[float, float, float]
    shape: tuple[int, int, int]
    value_count: int
    active_voxel_count: int
    min_value: float
    max_value: float
    estimated_volume_mm3: float
    surface_vertex_count: int = 0
    surface_face_count: int = 0
    metadata: dict[str, Any] = Field(default_factory=dict)


class VoxelRawLoadRequest(BaseModel):
    file_name: str = "volume.raw"
    contents_base64: str = Field(..., min_length=1)
    dimensions: tuple[int, int, int] | None = None
    voxel_size: tuple[float, float, float] = (1.0, 1.0, 1.0)
    scalar_type: str = "uint8"
    grid_level_set: bool = False
    auto_parameters: bool = False


class VoxelTiffLoadRequest(BaseModel):
    files: dict[str, str] = Field(..., min_length=2)
    voxel_size: tuple[float, float, float] = (1.0, 1.0, 1.0)
    grid_level_set: bool = False


class VoxelVolumeLoadResponse(BaseModel):
    version_id: str
    dimensions: tuple[int, int, int]
    voxel_size: tuple[float, float, float]
    grid_level_set: bool
    scalar_type: str
    value_count: int
    values: list[float] = Field(default_factory=list)
    min_value: float
    max_value: float
    default_iso_value: float | None = None
    metadata: dict[str, Any] = Field(default_factory=dict)


class VoxelBinaryOperationsRequest(BaseModel):
    left_values: list[float]
    right_values: list[float]
    shape: tuple[int, int, int]
    origin: tuple[float, float, float] = (0.0, 0.0, 0.0)
    voxel_size_mm: float = Field(default=1.0, gt=0)
    operation: str
    left_iso_value: float = 0.0
    right_iso_value: float = 0.0


class VoxelBinaryOperationsResponse(BaseModel):
    version_id: str
    operation: str
    shape: tuple[int, int, int]
    origin: tuple[float, float, float]
    voxel_size_mm: float
    values: list[float] = Field(default_factory=list)
    result_iso_value: float
    min_value: float
    max_value: float
    metadata: dict[str, Any] = Field(default_factory=dict)


class VoxelSliceRequest(BaseModel):
    values: list[float]
    shape: tuple[int, int, int]
    plane: str
    slice_index: int
    min_value: float
    max_value: float


class VoxelSliceResponse(BaseModel):
    version_id: str
    width: int
    height: int
    values: list[float] = Field(default_factory=list)
    normalized_values: list[float] = Field(default_factory=list)
    coordinates: list[tuple[int, int, int]] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)


class VoxelLineGraphRequest(BaseModel):
    values: list[float]
    shape: tuple[int, int, int]
    axis: str
    fixed_coordinate: tuple[int, int, int]


class VoxelLineGraphResponse(BaseModel):
    version_id: str
    axis: int
    positions: list[int] = Field(default_factory=list)
    voxel_indices: list[int] = Field(default_factory=list)
    coordinates: list[tuple[int, int, int]] = Field(default_factory=list)
    values: list[float] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)


class VoxelActiveBoxRequest(BaseModel):
    values: list[float]
    shape: tuple[int, int, int]
    min_corner: tuple[int, int, int]
    dimensions: tuple[int, int, int]


class VoxelActiveBoxResponse(BaseModel):
    version_id: str
    min_corner: tuple[int, int, int]
    dimensions: tuple[int, int, int]
    source_indices: list[int] = Field(default_factory=list)
    coordinates: list[tuple[int, int, int]] = Field(default_factory=list)
    values: list[float] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)


class VoxelPathRequest(BaseModel):
    values: list[float]
    shape: tuple[int, int, int]
    start: tuple[int, int, int]
    finish: tuple[int, int, int]
    metric: str = "difference"
    max_dist_ratio: float = 1.5
    plane: str = "none"
    quarters_mask: int = 15
    exponent_modifier: float = -1.0


class VoxelPathPayload(BaseModel):
    voxel_indices: list[int] = Field(default_factory=list)
    coordinates: list[tuple[int, int, int]] = Field(default_factory=list)
    total_metric: float


class VoxelPathResponse(VoxelPathPayload):
    version_id: str
    metadata: dict[str, Any] = Field(default_factory=dict)


class VoxelPathBuildFourRequest(BaseModel):
    values: list[float]
    shape: tuple[int, int, int]
    start: tuple[int, int, int]
    finish: tuple[int, int, int]
    metric: str = "exponent"
    max_dist_ratio: float = 1.5
    plane: str = "none"
    exponent_modifier: float = -1.0


class VoxelPathBuildFourEntry(BaseModel):
    quarters_mask: int
    path: VoxelPathPayload


class VoxelPathBuildFourResponse(BaseModel):
    version_id: str
    paths: list[VoxelPathBuildFourEntry] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)


class VoxelSegmentationRequest(BaseModel):
    values: list[float]
    shape: tuple[int, int, int]
    voxel_size: tuple[float, float, float] = (1.0, 1.0, 1.0)
    inside_seeds: list[tuple[int, int, int]]
    outside_seeds: list[tuple[int, int, int]] = Field(default_factory=list)
    exponent_modifier: float = 3000.0
    voxels_expansion: int = 25
    include_boundary_outside: bool = True


class VoxelSegmentationResponse(BaseModel):
    version_id: str
    vertex_count: int
    face_count: int
    bounds_min: tuple[float, float, float]
    bounds_max: tuple[float, float, float]
    vertices: list[tuple[float, float, float]] = Field(default_factory=list)
    faces: list[tuple[int, int, int]] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)


class VoxelMaskToMeshRequest(BaseModel):
    values: list[float]
    shape: tuple[int, int, int]
    voxel_size: tuple[float, float, float] = (1.0, 1.0, 1.0)
    mask_coordinates: list[tuple[int, int, int]]
    mask_expansion: int = 25
    smooth_band_radius: int = 3


class VoxelMaskToMeshResponse(VoxelSegmentationResponse):
    pass


class VoxelToMeshSimpleRequest(BaseModel):
    values: list[float]
    shape: tuple[int, int, int]
    voxel_size: tuple[float, float, float] = (1.0, 1.0, 1.0)
    iso_value: float | None = None
    grid_level_set: bool = False
    scalar_type: str = "float32"
    min_value: float | None = None
    max_value: float | None = None


class VoxelToMeshSimpleResponse(BaseModel):
    version_id: str
    vertex_count: int
    face_count: int
    bounds_min: tuple[float, float, float]
    bounds_max: tuple[float, float, float]
    vertices: list[tuple[float, float, float]] = Field(default_factory=list)
    faces: list[tuple[int, int, int]] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)


class VoxelToMeshDualRequest(VoxelToMeshSimpleRequest):
    values: list[float] = Field(default_factory=list)
    model_bytes_base64: str | None = None
    model_extension: str = ".vdb"
    adaptivity: float = Field(default=0.0, ge=0.0, le=1.0)
    relax_disoriented_triangles: bool = True
    max_faces: int | None = Field(default=None, ge=0)
    max_vertices: int | None = Field(default=None, ge=0)


class VoxelToMeshDualResponse(VoxelToMeshSimpleResponse):
    pass


class VoxelToMeshSmartRequest(VoxelToMeshSimpleRequest):
    iters: int = 30
    sample_points: int = 6
    degree: int = 3
    outlier_threshold: float = 1.0
    intermediate_smooth_force: float = 0.3
    preparation_smooth_force: float = 0.1
    smooth_shift_iterations: int = 15
    final_relax_iterations: int = 15
    final_relax_force: float = 0.01


class VoxelToMeshSmartResponse(VoxelToMeshSimpleResponse):
    pass


class VoxelVolumeRenderDataRequest(BaseModel):
    values: list[float]
    shape: tuple[int, int, int]
    voxel_size: tuple[float, float, float] = (1.0, 1.0, 1.0)
    active_min_corner: tuple[int, int, int] = (0, 0, 0)
    active_dimensions: tuple[int, int, int] | None = None
    source_min_value: float | None = None
    source_max_value: float | None = None


class VoxelVolumeRenderDataResponse(BaseModel):
    version_id: str
    dimensions: tuple[int, int, int]
    voxel_size: tuple[float, float, float]
    source_indices: list[int] = Field(default_factory=list)
    coordinates: list[tuple[int, int, int]] = Field(default_factory=list)
    values: list[float] = Field(default_factory=list)
    min_value: float
    max_value: float
    metadata: dict[str, Any] = Field(default_factory=dict)


class VoxelVolumeRenderLutRequest(BaseModel):
    lut_type: str = "rainbow"
    alpha_type: str = "constant"
    alpha_limit: int = Field(default=10, ge=0, le=255)
    one_color: tuple[int, int, int, int] | None = None


class VoxelVolumeRenderLutResponse(BaseModel):
    version_id: str
    lut_type: str
    alpha_type: str
    alpha_limit: int
    colors_rgba: list[tuple[int, int, int, int]] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)


class VoxelVolumeRenderRayRequest(BaseModel):
    values: list[float]
    shape: tuple[int, int, int]
    voxel_size: tuple[float, float, float] = (1.0, 1.0, 1.0)
    min_corner: tuple[int, int, int] = (0, 0, 0)
    ray_start: tuple[float, float, float]
    ray_direction: tuple[float, float, float]
    sampling_step: float
    min_value: float = 0.0
    max_value: float = 1.0
    lut_type: str = "rainbow"
    alpha_type: str = "constant"
    alpha_limit: int = Field(default=10, ge=0, le=255)
    one_color: tuple[int, int, int, int] | None = None
    clipping_plane: tuple[float, float, float, float] | None = None
    shading_mode: str = "none"
    light_pos_eye: tuple[float, float, float] | None = None
    ambient_strength: float = Field(default=0.1, ge=0)
    specular_strength: float = Field(default=0.5, ge=0)
    spec_exp: float = Field(default=35.0, ge=0)
    active_indices: list[int] | None = None
    max_steps: int = Field(default=4096, gt=0)


class VoxelVolumeRenderRayResponse(BaseModel):
    version_id: str
    color_rgba: list[float]
    first_opaque_world: tuple[float, float, float] | None = None
    visited_indices: list[int] = Field(default_factory=list)
    accepted_indices: list[int] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)


class VoxelBooleanRequest(BaseModel):
    other_version_id: str
    operation: Literal["union", "intersection", "difference"] = "union"
    voxel_size_mm: float = Field(default=0.5, gt=0)
    padding_mm: float | None = Field(default=None, gt=0)
    refine: bool = False


class VoxelBooleanResponse(BaseModel):
    version: ModelVersionSummary
    source_version_id: str
    other_version_id: str
    operation: str
    voxel_size_mm: float
    padding_mm: float | None = None
    refine: bool = False
    artifact_id: str
    artifact_url: str
    output_vertex_count: int
    output_face_count: int
    metadata: dict[str, Any] = Field(default_factory=dict)


class OffsetMeshRequest(BaseModel):
    offset_mm: float
    voxel_size_mm: float = Field(default=0.5, gt=0)
    padding_mm: float | None = Field(default=None, gt=0)
    refine: bool = False


class OffsetSmoothingRequest(BaseModel):
    distance_mm: float = Field(..., gt=0)
    voxel_size_mm: float = Field(default=0.5, gt=0)
    padding_mm: float | None = Field(default=None, gt=0)
    refine: bool = False


class ShellMeshRequest(BaseModel):
    wall_thickness_mm: float = Field(..., gt=0)
    voxel_size_mm: float = Field(default=0.5, gt=0)
    padding_mm: float | None = Field(default=None, gt=0)
    refine: bool = False


class ThickenMeshRequest(BaseModel):
    thickness_mm: float
    voxel_size_mm: float = Field(default=0.5, gt=0)
    padding_mm: float | None = Field(default=None, gt=0)
    refine: bool = False


class WeightedShellRegionWeight(BaseModel):
    region_id: str
    weight_mm: float


class WeightedShellRequest(BaseModel):
    offset_mm: float
    region_weights: list[WeightedShellRegionWeight] = Field(default_factory=list)
    voxel_size_mm: float = Field(default=0.5, gt=0)
    padding_mm: float | None = Field(default=None, gt=0)
    interpolation_distance_mm: float = Field(default=0.0, ge=0)
    refine: bool = False


class PartialOffsetRequest(BaseModel):
    offset_mm: float = Field(..., gt=0)
    region_ids: list[str] = Field(default_factory=list)
    voxel_size_mm: float = Field(default=0.5, gt=0)
    padding_mm: float | None = Field(default=None, gt=0)
    refine: bool = False


class OffsetVertsRequest(BaseModel):
    offset_mm: float
    region_ids: list[str] = Field(default_factory=list)


class OffsetShellMeshResponse(BaseModel):
    version: ModelVersionSummary
    source_version_id: str
    mode: Literal[
        "offset",
        "shell",
        "thicken",
        "weighted_shell",
        "partial_offset",
        "offset_verts",
        "expand_shrink",
        "shrink_expand",
    ]
    offset_mm: float | None = None
    distance_mm: float | None = None
    wall_thickness_mm: float | None = None
    thickness_mm: float | None = None
    region_weights: dict[str, float] | None = None
    selected_region_ids: list[str] | None = None
    voxel_size_mm: float
    padding_mm: float | None = None
    refine: bool = False
    artifact_id: str
    artifact_url: str
    output_vertex_count: int
    output_face_count: int
    metadata: dict[str, Any] = Field(default_factory=dict)


class MeshLibWorkbenchCommandCapability(BaseModel):
    command_id: str
    label: str
    group: Literal["file", "prepare", "modify", "inspect", "review", "runtime"]
    endpoint_url_key: str | None = None
    endpoint_url: str | None = None
    runtime_tool_id: str | None = None
    rust_backed: bool = False
    sdk_operations: list[str] = Field(default_factory=list)
    notes: list[str] = Field(default_factory=list)


class MeshLibOfficialParityFeature(BaseModel):
    official_feature_id: str
    label: str
    group: Literal[
        "file",
        "selection",
        "repair",
        "edit",
        "boolean",
        "offset",
        "inspect",
        "compare",
        "point_cloud",
        "voxels",
        "distance_map",
        "automation",
    ]
    status: Literal["implemented", "partial", "missing"]
    official_sources: list[str] = Field(default_factory=list)
    meshlib_source_paths: list[str] = Field(default_factory=list)
    rust_owner_modules: list[str] = Field(default_factory=list)
    bridge_modules: list[str] = Field(default_factory=list)
    backend_command_ids: list[str] = Field(default_factory=list)
    hosted_tool_ids: list[str] = Field(default_factory=list)
    validation_gates: list[str] = Field(default_factory=list)
    non_geometry_reason: str | None = None
    notes: list[str] = Field(default_factory=list)


class MeshLibWorkbenchManifest(BaseModel):
    version_id: str
    entry_html_url: str
    runtime_asset_base_url: str
    normalized_mesh_url: str | None = None
    meshlib_scene_object_url: str | None = None
    meshlib_scene_object_metadata: dict[str, Any] = Field(default_factory=dict)
    meshlib_scene_mru_url: str | None = None
    meshlib_scene_mru_metadata: dict[str, Any] = Field(default_factory=dict)
    texture_artifact_url: str | None = None
    texture_metadata: dict[str, Any] = Field(default_factory=dict)
    texture_artifacts: list[TextureArtifactManifest] = Field(default_factory=list)
    texture_per_face: list[int] = Field(default_factory=list)
    preview_low_url: str | None = None
    preview_high_url: str | None = None
    commit_endpoint_url: str
    selection_endpoint_url: str
    brush_endpoint_url: str
    measurement_endpoint_url: str
    mesh_cut_measure_topology_endpoint_url: str
    built_in_ui: list[str] = Field(default_factory=list)
    interactive_tools: list[str] = Field(default_factory=list)
    command_capabilities: list[MeshLibWorkbenchCommandCapability] = Field(default_factory=list)
    official_parity_inventory: list[MeshLibOfficialParityFeature] = Field(default_factory=list)
    feature_flags: dict[str, bool] = Field(default_factory=dict)
    notes: list[str] = Field(default_factory=list)


class CompareCacheEntry(BaseModel):
    other_version_id: str
    artifact_id: str
    created_at: datetime
    generated_by: str | None = None
    summary: "CompareResponse | None" = None


class InspectionSnapshotState(BaseModel):
    name: str = Field(..., min_length=1, max_length=120)
    axis_mode: Literal["auto", "manual"] = "auto"
    manual_axis: tuple[float, float, float] | None = None
    section_enabled: bool = False
    section_constant: float = 0.0
    selected_region_id: str | None = None
    selected_region_ids: list[str] = Field(default_factory=list)
    heatmap_enabled: bool = False
    compare_enabled: bool = False
    compare_target_version_id: str | None = None


class InspectionSnapshotResponse(InspectionSnapshotState):
    id: str
    version_id: str
    created_at: datetime


class SectionContourSegment(BaseModel):
    start: tuple[float, float, float]
    end: tuple[float, float, float]
    selected_region_hit: bool


class SectionContourPayload(BaseModel):
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
    segments: list[SectionContourSegment] = Field(default_factory=list)


class BranchVersionRequest(BaseModel):
    operation_label: str = Field(default="Restore Branch", min_length=1, max_length=255)


class JobResponse(BaseModel):
    id: str
    version_id: str
    operation_type: str
    status: str
    progress_pct: int
    error_code: str | None = None
    error_message: str | None = None
    result_json: dict[str, Any] | None = None
    started_at: datetime | None = None
    finished_at: datetime | None = None
    created_at: datetime


class JobEventResponse(BaseModel):
    id: str
    level: str
    message: str
    progress_pct: int | None = None
    created_at: datetime


class CreateModelResponse(BaseModel):
    model: ModelSummary
    version: ModelVersionSummary
    job: JobResponse | None = None


class RepairRequest(BaseModel):
    # Default (gentle) repair welds, fills holes, and orients faces, never
    # regressing the input. Opting into voxel_remesh additionally tries a voxel
    # rebuild that heals self-intersections the gentle path can't — at the cost of
    # resampling (lost fine detail), so it is user-chosen, not automatic.
    voxel_remesh: bool = False
    voxel_size_mm: float | None = Field(default=None, gt=0)


class ResizeRequest(BaseModel):
    target_ring_size_us: float = Field(..., ge=3, le=15)
    axis_mode: Literal["auto", "manual"] = "auto"
    manual_axis: tuple[float, float, float] | None = None
    preserve_head: bool = True


class HollowRequest(BaseModel):
    mode: Literal["fixed_thickness", "target_weight"] = "fixed_thickness"
    processing_mode: Literal["interactive", "full_resolution"] = "interactive"
    material: MaterialType = MaterialType.GOLD_18K
    wall_thickness_mm: float | None = Field(default=None, ge=0.3, le=5.0)
    target_weight_g: float | None = Field(default=None, ge=0.5)
    min_allowed_thickness_mm: float = Field(default=0.6, ge=0.2)
    protect_regions: list[Literal["head", "gem_seat", "ornament_relief", "inner_band"]] = Field(
        default_factory=lambda: ["head", "gem_seat", "ornament_relief"]
    )
    add_drain_holes: bool = False


class ThickenRequest(BaseModel):
    mode: Literal["global", "violations_only", "selected_region", "selected_regions"] = "violations_only"
    min_target_thickness_mm: float = Field(..., ge=0.3)
    region_id: str | None = None
    region_ids: list[str] = Field(default_factory=list)
    smoothing_pass: bool = True


class ScoopRequest(BaseModel):
    region_id: str
    depth_mm: float = Field(..., gt=0)
    falloff_mm: float = Field(..., gt=0)
    keep_min_thickness_mm: float = Field(default=0.6, ge=0.2)


class SmoothRequest(BaseModel):
    region_id: str | None = None
    region_ids: list[str] = Field(default_factory=list)
    iterations: int = Field(default=5, ge=1, le=50)
    strength: float = Field(default=0.5, ge=0.01, le=1.0)
    global_mode: bool = False


class DecimateRequest(BaseModel):
    strategy: Literal["minimize_error", "shortest_edge_first"] = "minimize_error"
    max_error: float = Field(default=1.7976931348623157e308, ge=0)
    target_face_count: int | None = Field(default=None, ge=1)
    target_face_ratio: float | None = Field(default=None, gt=0, le=1)
    max_edge_len: float | None = Field(default=None, gt=0)
    max_bd_shift: float | None = Field(default=None, ge=0)
    stabilizer: float = Field(default=0.001, ge=0)
    subdivide_parts: int = Field(default=1, ge=1, le=1024)
    decimate_between_parts: bool = True
    region_faces: list[int] = Field(default_factory=list)
    not_flippable_edges: list[tuple[int, int]] = Field(default_factory=list)
    edges_to_collapse: list[tuple[int, int]] = Field(default_factory=list)
    collapse_near_not_flippable: bool = False
    angle_weighted_dist_to_plane: bool = False
    max_deleted_vertices: int = Field(default=2_147_483_647, ge=1, le=2_147_483_647)
    max_deleted_faces: int = Field(default=2_147_483_647, ge=1, le=2_147_483_647)
    max_triangle_aspect_ratio: float = Field(default=20.0, ge=1)
    critical_tri_aspect_ratio: float | None = Field(default=None, ge=0)
    tiny_edge_length: float | None = Field(default=None, ge=0)
    max_angle_change: float | None = Field(default=None, ge=0)
    touch_near_bd_edges: bool = True
    touch_bd_verts: bool = True
    optimize_vertex_pos: bool = True
    pack_mesh: bool = True
    metadata: dict[str, Any] = Field(default_factory=dict)

    @model_validator(mode="after")
    def ensure_single_target_control(self) -> "DecimateRequest":
        if self.target_face_count is not None and self.target_face_ratio is not None:
            raise ValueError("target_face_count and target_face_ratio are mutually exclusive")
        return self


class SubdivideRequest(BaseModel):
    max_edge_len: float = Field(..., gt=0)
    max_edge_splits: int = Field(default=1000, ge=1, le=100_000)
    region_faces: list[int] = Field(default_factory=list)
    not_flippable_edges: list[tuple[int, int]] = Field(default_factory=list)
    subdivide_border: bool = True
    curvature_priority: float = Field(default=0.0, ge=0)
    project_on_original_mesh: bool = False
    smooth_mode: bool = False
    min_sharp_dihedral_angle: float = Field(default=0.5235987755982989, gt=0)
    max_tri_aspect_ratio: float = Field(default=0.0, ge=0)
    max_splittable_tri_aspect_ratio: float | None = Field(default=None, ge=0)
    max_deviation_after_flip: float | None = Field(default=None, ge=0)
    max_angle_change_after_flip: float | None = Field(default=None, ge=0)
    critical_tri_aspect_ratio_flip: float | None = Field(default=None, ge=0)


class MakeDeloneRequest(BaseModel):
    num_iters: int = Field(default=1, ge=1, le=1000)
    region_faces: list[int] = Field(default_factory=list)
    max_deviation_after_flip: float | None = Field(default=None, ge=0)
    max_angle_change: float | None = Field(default=None, ge=0)
    critical_tri_aspect_ratio: float | None = Field(default=None, ge=0)
    not_flippable_edges: list[tuple[int, int]] = Field(default_factory=list)
    vert_region: list[int] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)


class CompareRequest(BaseModel):
    other_version_id: str


class CollisionDetectRequest(BaseModel):
    other_version_id: str
    first_intersection_only: bool = False
    max_pairs: int = Field(default=1000, ge=1, le=50000)
    epsilon: float = Field(default=1e-8, gt=0)


class CollisionFacePair(BaseModel):
    first_face: int
    second_face: int
    intersection_count: int


class CollisionDetectResponse(BaseModel):
    version_id: str
    other_version_id: str
    colliding: bool
    pair_count: int
    first_face_indices: list[int] = Field(default_factory=list)
    second_face_indices: list[int] = Field(default_factory=list)
    pairs: list[CollisionFacePair] = Field(default_factory=list)
    truncated: bool = False
    metadata: dict[str, Any] = Field(default_factory=dict)


class ExactBooleanRequest(BaseModel):
    other_version_id: str
    operation: Literal[
        "union",
        "intersection",
        "difference",
        "difference_ab",
        "difference_ba",
        "inside_a",
        "inside_b",
        "outside_a",
        "outside_b",
    ] = "difference"
    epsilon: float = Field(default=1e-8, gt=0)


class ExactBooleanResponse(BaseModel):
    version: ModelVersionSummary
    source_version_id: str
    other_version_id: str
    operation: str
    artifact_id: str
    artifact_url: str
    output_vertex_count: int
    output_face_count: int
    diagnostics: dict[str, Any] = Field(default_factory=dict)
    metadata: dict[str, Any] = Field(default_factory=dict)


class MakeManufacturableRequest(BaseModel):
    material: MaterialType = MaterialType.GOLD_18K
    target_ring_size_us: float | None = Field(default=None, ge=3, le=15)
    target_weight_g: float | None = Field(default=None, ge=0.5)
    min_allowed_thickness_mm: float = Field(default=0.6, ge=0.2)


class CompareResponse(BaseModel):
    version_id: str
    other_version_id: str
    volume_delta_mm3: float
    weight_delta_g: float
    bbox_delta_mm: tuple[float, float, float]
    min_signed_distance_mm: float | None = None
    max_signed_distance_mm: float | None = None
    mean_signed_distance_mm: float | None = None


class VersionDetailResponse(BaseModel):
    version: ModelVersionSummary
    artifacts: list[ArtifactSummary]
    latest_snapshot: ManufacturabilitySnapshot | None = None
