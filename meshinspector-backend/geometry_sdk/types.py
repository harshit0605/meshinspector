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
class HoleFillReport:
    input_holes: int
    filled_holes: int
    added_vertices: int
    added_faces: int
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
class SDFGrid:
    origin: tuple[float, float, float]
    voxel_size_mm: float
    shape: tuple[int, int, int]
    values: np.ndarray

    def points(self) -> np.ndarray:
        axes = [
            self.origin[axis] + np.arange(self.shape[axis], dtype=np.float64) * self.voxel_size_mm
            for axis in range(3)
        ]
        grid = np.meshgrid(*axes, indexing="ij")
        return np.stack(grid, axis=-1).reshape(-1, 3)

    def point_to_grid(self, points: np.ndarray) -> np.ndarray:
        return (np.asarray(points, dtype=np.float64) - np.asarray(self.origin, dtype=np.float64)) / self.voxel_size_mm


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
