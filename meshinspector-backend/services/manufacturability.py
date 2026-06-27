"""Manufacturability snapshot orchestration."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

import numpy as np

from core.config import settings
from domain.schemas import (
    DimensionsSnapshot,
    ManufacturabilitySnapshot,
    MaterialType,
    MaterialWeightEntry,
    MeshHealthSnapshot,
    RegionManifestEntry,
    ThicknessSnapshot,
)
from geometry_sdk import RegionEntry, ThicknessSummary, default_sdk
from utils.units import mm3_to_grams


@dataclass(slots=True)
class ManufacturabilityArtifacts:
    thickness_scalar_path: Path
    region_json_path: Path


def _build_recommendations(
    health: MeshHealthSnapshot,
    dimensions: DimensionsSnapshot,
    thickness: ThicknessSnapshot,
    regions: list,
    *,
    thickness_deferred: bool = False,
) -> list[str]:
    recommendations: list[str] = []
    if not health.is_closed or health.holes_count > 0:
        recommendations.append("Run auto repair before any hollowing or boolean operations.")
    if health.self_intersections > 0:
        recommendations.append("Repair self-intersections before export.")
    if health.disconnected_shells > 0:
        recommendations.append(
            f"Result has {health.disconnected_shells} extra disconnected shell(s); it is not a single "
            "solid. Prune stray fragments or re-run the operation at a finer resolution before export."
        )
    if thickness_deferred:
        recommendations.append("Thickness analysis deferred for this large mesh; use focused Measure/Inspect before casting.")
    elif thickness.min_mm is None:
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


def _to_region_manifest_entry(region: RegionEntry) -> RegionManifestEntry:
    return RegionManifestEntry(
        region_id=region.region_id,
        label=region.label,
        vertex_count=int(region.vertex_indices.size),
        coverage_pct=round(float(region.coverage_pct), 2),
        min_thickness_mm=round(float(region.min_thickness_mm), 4) if region.min_thickness_mm is not None else None,
        avg_thickness_mm=round(float(region.avg_thickness_mm), 4) if region.avg_thickness_mm is not None else None,
        violation_count=int(region.violation_count),
        protected_by_default=region.protected_by_default,
        allowed_operations=list(region.allowed_operations),
        centroid_mm=(
            round(float(region.centroid_mm[0]), 4),
            round(float(region.centroid_mm[1]), 4),
            round(float(region.centroid_mm[2]), 4),
        )
        if region.centroid_mm is not None
        else None,
    )


def _write_region_payload(output_dir: Path, axis: tuple[float, float, float], regions: list[RegionEntry]) -> tuple[Path, list[RegionManifestEntry]]:
    manifest = [_to_region_manifest_entry(region) for region in regions]
    payload = {
        "ring_axis": [round(float(axis[0]), 6), round(float(axis[1]), 6), round(float(axis[2]), 6)],
        "regions": [
            {
                **entry.model_dump(mode="json"),
                "vertex_indices": np.asarray(region.vertex_indices, dtype=np.int64).astype(int).tolist(),
            }
            for entry, region in zip(manifest, regions, strict=True)
        ],
    }
    region_json_path = output_dir / "regions.json"
    region_json_path.write_text(json.dumps(payload), encoding="utf-8")
    return region_json_path, manifest


def compute_manufacturability_snapshot(
    mesh_path: str | Path,
    output_dir: str | Path,
    threshold_mm: float | None = None,
) -> tuple[ManufacturabilitySnapshot, ManufacturabilityArtifacts]:
    """Compute a full manufacturability snapshot for a normalized mesh artifact."""
    mesh_path = Path(mesh_path)
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    mesh = default_sdk.load_mesh(mesh_path)
    stats = default_sdk.stats(mesh)
    volume_mm3 = abs(float(stats.volume_mm3))

    health_raw = default_sdk.service_health(mesh)
    disconnected_shells = max(stats.connected_components - 1, 0)
    raw_score = float(health_raw.health_score) if health_raw.health_score is not None else 0.0
    # A result split into multiple disconnected shells is not a single manufacturable
    # solid, so it must not read as a perfect score. The Rust health score doesn't
    # account for component count; penalize it here (≈25 points per extra shell) so a
    # fragmented ring reports honestly instead of "health 100".
    health_score = raw_score if disconnected_shells == 0 else min(raw_score, max(0.0, 100.0 - 25.0 * disconnected_shells))
    health = MeshHealthSnapshot(
        is_closed=health_raw.is_closed,
        holes_count=health_raw.holes_count,
        self_intersections=health_raw.self_intersections,
        disconnected_shells=disconnected_shells,
        health_score=health_score,
    )

    measurement = default_sdk.measure_ring(mesh)
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

    effective_threshold = threshold_mm or settings.DEFAULT_MIN_THICKNESS_MM
    thickness_deferred = (
        settings.MANUFACTURABILITY_THICKNESS_MAX_VERTICES > 0
        and mesh.vertex_count > settings.MANUFACTURABILITY_THICKNESS_MAX_VERTICES
    )
    if thickness_deferred:
        thickness_scalars = np.full(mesh.vertex_count, np.nan, dtype=np.float32)
        thickness_summary = ThicknessSummary(
            min_mm=None,
            avg_mm=None,
            max_mm=None,
            valid_vertex_count=0,
            violation_count=0,
        )
    else:
        thickness_scalars, thickness_summary = default_sdk.service_thickness(mesh, threshold_mm=effective_threshold)
    thickness_scalar_path = default_sdk.save_thickness_npz(
        output_dir / "thickness.npz",
        thickness_scalars,
        vertex_count=mesh.vertex_count,
        threshold_mm=effective_threshold,
    )
    thickness = ThicknessSnapshot(
        min_mm=thickness_summary.min_mm,
        avg_mm=thickness_summary.avg_mm,
        max_mm=thickness_summary.max_mm,
        violation_count=thickness_summary.violation_count,
        threshold_mm=effective_threshold,
        scalar_field_artifact_id=None,
    )
    regions = default_sdk.detect_ring_regions(
        mesh,
        measurement,
        thickness=None if thickness_deferred else thickness_scalars,
        threshold_mm=effective_threshold,
    )
    region_json_path, region_manifest = _write_region_payload(output_dir, measurement.ring_axis, regions)

    recommendations = _build_recommendations(health, dimensions, thickness, region_manifest, thickness_deferred=thickness_deferred)
    export_ready = (
        health.is_closed
        and health.disconnected_shells == 0
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
        regions=region_manifest,
        recommendations=recommendations,
        export_ready=export_ready,
    )
    return snapshot, ManufacturabilityArtifacts(
        thickness_scalar_path=thickness_scalar_path,
        region_json_path=region_json_path,
    )
