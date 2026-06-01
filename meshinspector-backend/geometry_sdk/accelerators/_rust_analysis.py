from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument, ThicknessSummary, VersionCompareSummary


def _require_rust_kernel(name: str):
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None
    if not hasattr(_common._rs, name):
        if mode == "rust":
            raise RuntimeError(f"GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs does not expose {name}")
        return None
    return getattr(_common._rs, name)


def summarize_thickness(thickness: np.ndarray, *, threshold_mm: float = 0.6) -> ThicknessSummary | None:
    kernel = _require_rust_kernel("summarize_thickness")
    if kernel is None:
        return None
    values = np.asarray(thickness, dtype=np.float32).reshape(-1)
    payload: dict[str, Any] = kernel(values, float(threshold_mm))
    return ThicknessSummary(
        min_mm=None if payload["min_mm"] is None else float(payload["min_mm"]),
        avg_mm=None if payload["avg_mm"] is None else float(payload["avg_mm"]),
        max_mm=None if payload["max_mm"] is None else float(payload["max_mm"]),
        valid_vertex_count=int(payload["valid_vertex_count"]),
        violation_count=int(payload["violation_count"]),
    )


def nearest_surface_distances(source: MeshDocument, target: MeshDocument) -> np.ndarray | None:
    kernel = _require_rust_kernel("nearest_surface_distances")
    if kernel is None:
        return None
    return np.asarray(kernel(source.vertices, target.vertices, target.faces), dtype=np.float32)


def nearest_vertex_distances(source: MeshDocument, target: MeshDocument) -> np.ndarray | None:
    kernel = _require_rust_kernel("nearest_vertex_distances")
    if kernel is None:
        return None
    return np.asarray(kernel(source.vertices, target.vertices), dtype=np.float32)


def signed_surface_distances(source: MeshDocument, target: MeshDocument) -> np.ndarray | None:
    kernel = _require_rust_kernel("signed_surface_distances")
    if kernel is None:
        return None
    return np.asarray(kernel(source.vertices, target.vertices, target.faces), dtype=np.float32)


def version_compare_distances(source: MeshDocument, target: MeshDocument) -> np.ndarray | None:
    kernel = _require_rust_kernel("version_compare_distances")
    if kernel is None:
        return None
    return np.asarray(kernel(source.vertices, target.vertices, target.faces), dtype=np.float32)


def service_compare_distances(source: MeshDocument, other: MeshDocument) -> np.ndarray | None:
    kernel = _require_rust_kernel("service_compare_distances")
    if kernel is None:
        return None
    return np.asarray(kernel(source.vertices, source.faces, other.vertices), dtype=np.float32)


def _distance_summary_payload(payload: dict[str, Any], *, signed: bool) -> dict[str, float | None]:
    if signed:
        return {
            "min_signed_distance_mm": None if payload["min_mm"] is None else float(payload["min_mm"]),
            "max_signed_distance_mm": None if payload["max_mm"] is None else float(payload["max_mm"]),
            "mean_signed_distance_mm": None if payload["mean_mm"] is None else float(payload["mean_mm"]),
        }
    return {
        "min_distance_mm": None if payload["min_mm"] is None else float(payload["min_mm"]),
        "max_distance_mm": None if payload["max_mm"] is None else float(payload["max_mm"]),
        "mean_distance_mm": None if payload["mean_mm"] is None else float(payload["mean_mm"]),
    }


def compare_summary(source: MeshDocument, target: MeshDocument) -> dict[str, float | None] | None:
    kernel = _require_rust_kernel("compare_summary")
    if kernel is None:
        return None
    return _distance_summary_payload(kernel(source.vertices, target.vertices, target.faces), signed=False)


def signed_compare_summary(source: MeshDocument, target: MeshDocument) -> dict[str, float | None] | None:
    kernel = _require_rust_kernel("signed_compare_summary")
    if kernel is None:
        return None
    return _distance_summary_payload(kernel(source.vertices, target.vertices, target.faces), signed=True)


def version_compare_summary(source: MeshDocument, target: MeshDocument) -> VersionCompareSummary | None:
    kernel = _require_rust_kernel("version_compare_summary")
    if kernel is None:
        return None
    payload: dict[str, Any] = kernel(source.vertices, source.faces, target.vertices, target.faces)
    return VersionCompareSummary(
        volume_delta_mm3=float(payload["volume_delta_mm3"]),
        bbox_delta_mm=tuple(float(value) for value in payload["bbox_delta_mm"]),
        min_signed_distance_mm=None if payload["min_signed_distance_mm"] is None else float(payload["min_signed_distance_mm"]),
        max_signed_distance_mm=None if payload["max_signed_distance_mm"] is None else float(payload["max_signed_distance_mm"]),
        mean_signed_distance_mm=None if payload["mean_signed_distance_mm"] is None else float(payload["mean_signed_distance_mm"]),
    )


def service_compare_summary(source: MeshDocument, other: MeshDocument) -> VersionCompareSummary | None:
    kernel = _require_rust_kernel("service_compare_summary")
    if kernel is None:
        return None
    payload: dict[str, Any] = kernel(source.vertices, source.faces, other.vertices, other.faces)
    return VersionCompareSummary(
        volume_delta_mm3=float(payload["volume_delta_mm3"]),
        bbox_delta_mm=tuple(float(value) for value in payload["bbox_delta_mm"]),
        min_signed_distance_mm=None if payload["min_signed_distance_mm"] is None else float(payload["min_signed_distance_mm"]),
        max_signed_distance_mm=None if payload["max_signed_distance_mm"] is None else float(payload["max_signed_distance_mm"]),
        mean_signed_distance_mm=None if payload["mean_signed_distance_mm"] is None else float(payload["mean_signed_distance_mm"]),
    )
