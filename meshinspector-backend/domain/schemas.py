"""Pydantic schemas for the production-grade API."""

from __future__ import annotations

from datetime import datetime
from enum import Enum
from typing import Any, Literal

from pydantic import BaseModel, Field


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
    REPAIR = "repair"
    RESIZE = "resize"
    HOLLOW = "hollow"
    THICKEN = "thicken"
    SCOOP = "scoop"
    SMOOTH = "smooth"
    COMPARE = "compare"
    MAKE_MANUFACTURABLE = "make_manufacturable"


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


class ViewerManifest(BaseModel):
    version_id: str
    preview_low_url: str | None = None
    preview_high_url: str | None = None
    normalized_mesh_url: str | None = None
    thickness_artifact_url: str | None = None
    region_artifact_url: str | None = None
    bounding_box: tuple[float, float, float]
    default_material: MaterialType = MaterialType.GOLD_18K
    available_overlays: list[str] = Field(default_factory=list)
    region_manifest: list[RegionManifestEntry] = Field(default_factory=list)
    measurements_summary: dict[str, Any] = Field(default_factory=dict)
    can_edit: bool = True
    needs_axis_confirmation: bool = False


class CompareCacheEntry(BaseModel):
    other_version_id: str
    artifact_id: str
    created_at: datetime
    generated_by: str | None = None


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


class ResizeRequest(BaseModel):
    target_ring_size_us: float = Field(..., ge=3, le=15)
    axis_mode: Literal["auto", "manual"] = "auto"
    manual_axis: tuple[float, float, float] | None = None
    preserve_head: bool = True


class HollowRequest(BaseModel):
    mode: Literal["fixed_thickness", "target_weight"] = "fixed_thickness"
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


class CompareRequest(BaseModel):
    other_version_id: str


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
