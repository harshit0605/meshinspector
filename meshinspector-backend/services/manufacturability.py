"""Manufacturability snapshot orchestration."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import numpy as np
import trimesh

from core.config import settings
from domain.schemas import (
    DimensionsSnapshot,
    ManufacturabilitySnapshot,
    MaterialType,
    MaterialWeightEntry,
    MeshHealthSnapshot,
    ThicknessSnapshot,
)
from services.convert import normalize_mesh_to_mm
from services.health import check_mesh_health
from services.measure_ring import measure_ring_from_path
from services.regions import RegionArtifacts, detect_ring_regions
from services.thickness_meshlib import ThicknessResult, compute_thickness_meshlib
from utils.units import MATERIAL_DENSITIES, mm3_to_grams


@dataclass(slots=True)
class ManufacturabilityArtifacts:
    thickness_scalar_path: Path
    region_json_path: Path


def _count_disconnected_shells(mesh: trimesh.Trimesh) -> int:
    """Count connected face components without optional trimesh extras."""
    if not mesh.faces.size:
        return 0
    if len(mesh.faces) == 1:
        return 1

    components = trimesh.graph.connected_components(
        mesh.face_adjacency,
        nodes=np.arange(len(mesh.faces)),
        engine="scipy",
    )
    return len(components)


def _build_recommendations(
    health: MeshHealthSnapshot,
    dimensions: DimensionsSnapshot,
    thickness: ThicknessSnapshot,
    regions: list,
) -> list[str]:
    recommendations: list[str] = []
    if not health.is_closed or health.holes_count > 0:
        recommendations.append("Run auto repair before any hollowing or boolean operations.")
    if health.self_intersections > 0:
        recommendations.append("Repair self-intersections before export.")
    if thickness.min_mm is None:
        recommendations.append("Thickness analysis failed; inspect the mesh manually.")
    elif thickness.min_mm < thickness.threshold_mm:
        recommendations.append("Fix thin regions before casting.")
    if dimensions.needs_axis_confirmation:
        recommendations.append("Confirm the detected ring axis before resizing.")
    protected_violations = [region.label for region in regions if region.protected_by_default and region.violation_count > 0]
    if protected_violations:
        recommendations.append(f"Protected detail regions need attention: {', '.join(protected_violations)}.")
    if not recommendations:
        recommendations.append("Mesh is ready for guided manufacturing workflows.")
    return recommendations


def _material_weight_table(volume_mm3: float) -> dict[MaterialType, MaterialWeightEntry]:
    table: dict[MaterialType, MaterialWeightEntry] = {}
    for material in MaterialType:
        table[material] = MaterialWeightEntry(
            volume_mm3=round(volume_mm3, 3),
            weight_g=round(mm3_to_grams(volume_mm3, material.value), 3),
        )
    return table


def compute_manufacturability_snapshot(
    mesh_path: str | Path,
    output_dir: str | Path,
    threshold_mm: float | None = None,
) -> tuple[ManufacturabilitySnapshot, ManufacturabilityArtifacts]:
    """Compute a full manufacturability snapshot for a normalized mesh artifact."""
    mesh_path = Path(mesh_path)
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    mesh = normalize_mesh_to_mm(mesh_path)
    volume_mm3 = abs(float(mesh.volume))
    disconnected_shells = _count_disconnected_shells(mesh)

    health_raw = check_mesh_health(mesh_path)
    health = MeshHealthSnapshot(
        is_closed=health_raw["is_closed"],
        holes_count=health_raw["holes_count"],
        self_intersections=health_raw["self_intersections"],
        disconnected_shells=max(disconnected_shells - 1, 0),
        health_score=health_raw["health_score"],
    )

    measurement = measure_ring_from_path(mesh_path)
    dimensions = DimensionsSnapshot(
        ring_axis=measurement.ring_axis,
        ring_axis_confidence=measurement.ring_axis_confidence,
        estimated_ring_size_us=measurement.estimated_ring_size_us,
        inner_diameter_mm=measurement.inner_diameter_mm,
        band_width_min_mm=measurement.band_width_min_mm,
        band_width_max_mm=measurement.band_width_max_mm,
        head_height_mm=measurement.head_height_mm,
        bbox_mm=measurement.bbox_mm,
        needs_axis_confirmation=measurement.needs_axis_confirmation,
    )

    thickness_result: ThicknessResult = compute_thickness_meshlib(
        mesh_path,
        output_dir,
        threshold_mm=threshold_mm or settings.DEFAULT_MIN_THICKNESS_MM,
    )
    thickness_payload = np.load(thickness_result.scalar_field_path)
    thickness_scalars = thickness_payload["thickness"].astype(np.float32)
    thickness = ThicknessSnapshot(
        min_mm=thickness_result.min_mm,
        avg_mm=thickness_result.avg_mm,
        max_mm=thickness_result.max_mm,
        violation_count=thickness_result.violation_count,
        threshold_mm=thickness_result.threshold_mm,
        scalar_field_artifact_id=None,
    )
    region_artifacts: RegionArtifacts = detect_ring_regions(
        mesh,
        measurement,
        thickness_scalars,
        output_dir,
        threshold_mm=thickness_result.threshold_mm,
    )

    recommendations = _build_recommendations(health, dimensions, thickness, region_artifacts.manifest)
    export_ready = (
        health.is_closed
        and health.self_intersections == 0
        and thickness.min_mm is not None
        and thickness.min_mm >= thickness.threshold_mm
    )
    snapshot = ManufacturabilitySnapshot(
        version_id="",
        mesh_health=health,
        dimensions=dimensions,
        material_weight=_material_weight_table(volume_mm3),
        thickness=thickness,
        regions=region_artifacts.manifest,
        recommendations=recommendations,
        export_ready=export_ready,
    )
    return snapshot, ManufacturabilityArtifacts(
        thickness_scalar_path=thickness_result.scalar_field_path,
        region_json_path=region_artifacts.region_json_path,
    )
