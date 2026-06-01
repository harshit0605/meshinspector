from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import (
    ManufacturabilityReport,
    MaterialWeightEntry,
    MeshDocument,
    MeshHealth,
    MeshStats,
    RegionEntry,
    RingMeasurement,
    ThicknessSummary,
)


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def _health_from_payload(payload: dict[str, Any]) -> MeshHealth:
    return MeshHealth(
        is_closed=bool(payload["is_closed"]),
        holes_count=int(payload["holes_count"]),
        boundary_edge_count=int(payload["boundary_edge_count"]),
        nonmanifold_edge_count=int(payload["nonmanifold_edge_count"]),
        self_intersections=None if payload["self_intersections"] is None else int(payload["self_intersections"]),
        self_intersections_available=bool(payload["self_intersections_available"]),
    )


def _stats_from_payload(payload: dict[str, Any]) -> MeshStats:
    return MeshStats(
        bbox_min=tuple(float(value) for value in payload["bbox_min"]),
        bbox_max=tuple(float(value) for value in payload["bbox_max"]),
        bbox_size=tuple(float(value) for value in payload["bbox_size"]),
        surface_area_mm2=float(payload["surface_area_mm2"]),
        volume_mm3=float(payload["volume_mm3"]),
        vertex_count=int(payload["vertex_count"]),
        face_count=int(payload["face_count"]),
        connected_components=int(payload["connected_components"]),
        boundary_edge_count=int(payload["boundary_edge_count"]),
    )


def _measurement_from_payload(payload: dict[str, Any]) -> RingMeasurement:
    return RingMeasurement(
        ring_axis=tuple(float(value) for value in payload["ring_axis"]),
        ring_axis_confidence=float(payload["ring_axis_confidence"]),
        estimated_ring_size_us=None
        if payload["estimated_ring_size_us"] is None
        else float(payload["estimated_ring_size_us"]),
        inner_diameter_mm=None if payload["inner_diameter_mm"] is None else float(payload["inner_diameter_mm"]),
        band_width_min_mm=None if payload["band_width_min_mm"] is None else float(payload["band_width_min_mm"]),
        band_width_max_mm=None if payload["band_width_max_mm"] is None else float(payload["band_width_max_mm"]),
        head_height_mm=None if payload["head_height_mm"] is None else float(payload["head_height_mm"]),
        bbox_mm=tuple(float(value) for value in payload["bbox_mm"]),
        needs_axis_confirmation=bool(payload["needs_axis_confirmation"]),
    )


def _thickness_from_payload(payload: dict[str, Any]) -> ThicknessSummary:
    return ThicknessSummary(
        min_mm=None if payload["min_mm"] is None else float(payload["min_mm"]),
        avg_mm=None if payload["avg_mm"] is None else float(payload["avg_mm"]),
        max_mm=None if payload["max_mm"] is None else float(payload["max_mm"]),
        valid_vertex_count=int(payload["valid_vertex_count"]),
        violation_count=int(payload["violation_count"]),
    )


def _region_from_payload(payload: dict[str, Any]) -> RegionEntry:
    return RegionEntry(
        region_id=str(payload["region_id"]),
        label=str(payload["label"]),
        vertex_indices=np.asarray(payload["vertex_indices"], dtype=np.int32),
        coverage_pct=float(payload["coverage_pct"]),
        protected_by_default=bool(payload["protected_by_default"]),
        allowed_operations=[str(value) for value in payload["allowed_operations"]],
        min_thickness_mm=None if payload["min_thickness_mm"] is None else float(payload["min_thickness_mm"]),
        avg_thickness_mm=None if payload["avg_thickness_mm"] is None else float(payload["avg_thickness_mm"]),
        violation_count=int(payload["violation_count"]),
        centroid_mm=None
        if payload["centroid_mm"] is None
        else tuple(float(value) for value in payload["centroid_mm"]),
    )


def _material_weights_from_payload(payload: dict[str, dict[str, Any]]) -> dict[str, MaterialWeightEntry]:
    return {
        str(material): MaterialWeightEntry(
            volume_mm3=float(entry["volume_mm3"]),
            weight_g=float(entry["weight_g"]),
        )
        for material, entry in payload.items()
    }


def health_score(health: MeshHealth) -> int:
    kernel = _require_rust_kernel("health_score")
    return int(
        kernel(
            bool(health.is_closed),
            int(health.holes_count),
            int(health.boundary_edge_count),
            int(health.nonmanifold_edge_count),
            None if health.self_intersections is None else int(health.self_intersections),
            bool(health.self_intersections_available),
        )
    )


def build_recommendations(
    health: MeshHealth,
    measurement: RingMeasurement,
    thickness: ThicknessSummary,
    regions: list[RegionEntry],
    *,
    threshold_mm: float,
) -> list[str]:
    kernel = _require_rust_kernel("build_recommendations")
    protected_labels = [
        region.label for region in regions if region.protected_by_default and region.violation_count > 0
    ]
    return [
        str(value)
        for value in kernel(
            bool(health.is_closed),
            int(health.holes_count),
            int(health.boundary_edge_count),
            int(health.nonmanifold_edge_count),
            None if health.self_intersections is None else int(health.self_intersections),
            bool(measurement.needs_axis_confirmation),
            None if thickness.min_mm is None else float(thickness.min_mm),
            [str(label) for label in protected_labels],
            float(threshold_mm),
        )
    ]


def compute_manufacturability_report(mesh: MeshDocument, *, threshold_mm: float = 0.6) -> ManufacturabilityReport:
    kernel = _require_rust_kernel("compute_manufacturability_report")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, float(threshold_mm))
    return ManufacturabilityReport(
        health=_health_from_payload(payload["health"]),
        stats=_stats_from_payload(payload["stats"]),
        ring_measurement=_measurement_from_payload(payload["ring_measurement"]),
        thickness=_thickness_from_payload(payload["thickness"]),
        regions=[_region_from_payload(region) for region in payload["regions"]],
        material_weights=_material_weights_from_payload(payload["material_weights"]),
        recommendations=[str(value) for value in payload["recommendations"]],
        export_ready=bool(payload["export_ready"]),
        health_score=int(payload["health_score"]),
    )
