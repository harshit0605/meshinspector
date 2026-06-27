from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument, RegionEntry, RingMeasurement


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def ring_diameter_for_size(size: float) -> float | None:
    kernel = _require_rust_kernel("ring_diameter_for_size")
    return float(kernel(float(size)))


def closest_ring_size(inner_diameter_mm: float | None) -> float | None:
    if inner_diameter_mm is None:
        return None
    kernel = _require_rust_kernel("closest_ring_size")
    value = kernel(float(inner_diameter_mm))
    return None if value is None else float(value)


def measure_ring(mesh: MeshDocument, *, axis_override: Any = None) -> RingMeasurement | None:
    kernel = _require_rust_kernel("measure_ring")
    axis = None if axis_override is None else np.asarray(axis_override, dtype=np.float64)
    payload: dict[str, Any] = kernel(mesh.vertices, axis)
    return RingMeasurement(
        ring_axis=tuple(float(x) for x in payload["ring_axis"]),
        ring_axis_confidence=float(payload["ring_axis_confidence"]),
        estimated_ring_size_us=None
        if payload["estimated_ring_size_us"] is None
        else float(payload["estimated_ring_size_us"]),
        inner_diameter_mm=None if payload["inner_diameter_mm"] is None else float(payload["inner_diameter_mm"]),
        band_width_min_mm=None
        if payload["band_width_min_mm"] is None
        else float(payload["band_width_min_mm"]),
        band_width_max_mm=None
        if payload["band_width_max_mm"] is None
        else float(payload["band_width_max_mm"]),
        head_height_mm=None if payload["head_height_mm"] is None else float(payload["head_height_mm"]),
        bbox_mm=tuple(float(x) for x in payload["bbox_mm"]),
        needs_axis_confirmation=bool(payload["needs_axis_confirmation"]),
    )


def detect_ring_regions(
    mesh: MeshDocument,
    measurement: RingMeasurement,
    *,
    thickness: Any = None,
    threshold_mm: float = 0.6,
) -> list[RegionEntry]:
    kernel = _require_rust_kernel("detect_ring_regions")
    values = None if thickness is None else np.asarray(thickness, dtype=np.float32)
    payloads: list[dict[str, Any]] = kernel(
        mesh.vertices,
        mesh.faces,
        np.asarray(measurement.ring_axis, dtype=np.float64),
        values,
        float(threshold_mm),
    )
    return [_region_from_payload(payload) for payload in payloads]


def _region_from_payload(payload: dict[str, Any]) -> RegionEntry:
    centroid = payload["centroid_mm"]
    return RegionEntry(
        region_id=str(payload["region_id"]),
        label=str(payload["label"]),
        vertex_indices=np.asarray(payload["vertex_indices"], dtype=np.int32),
        coverage_pct=float(payload["coverage_pct"]),
        protected_by_default=bool(payload["protected_by_default"]),
        allowed_operations=[str(operation) for operation in payload["allowed_operations"]],
        min_thickness_mm=None
        if payload["min_thickness_mm"] is None
        else float(payload["min_thickness_mm"]),
        avg_thickness_mm=None
        if payload["avg_thickness_mm"] is None
        else float(payload["avg_thickness_mm"]),
        violation_count=int(payload["violation_count"]),
        centroid_mm=None if centroid is None else tuple(float(value) for value in centroid),
    )
