from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument, SectionContourPayload, SectionContourSegment, ThicknessSummary, VersionCompareSummary


def _require_rust_kernel(name: str):
    _common.accelerator_mode()
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def summarize_thickness(thickness: np.ndarray, *, threshold_mm: float = 0.6) -> ThicknessSummary:
    kernel = _require_rust_kernel("summarize_thickness")
    values = np.asarray(thickness, dtype=np.float32).reshape(-1)
    payload: dict[str, Any] = kernel(values, float(threshold_mm))
    return ThicknessSummary(
        min_mm=None if payload["min_mm"] is None else float(payload["min_mm"]),
        avg_mm=None if payload["avg_mm"] is None else float(payload["avg_mm"]),
        max_mm=None if payload["max_mm"] is None else float(payload["max_mm"]),
        valid_vertex_count=int(payload["valid_vertex_count"]),
        violation_count=int(payload["violation_count"]),
    )


def scalar_overlay_payload(
    values: np.ndarray,
    *,
    overlay_type: str,
    center_value: float,
    threshold_mm: float | None = None,
    max_abs_value: float = 1_000_000.0,
) -> dict[str, Any]:
    kernel = _require_rust_kernel("scalar_overlay_payload")
    field = np.asarray(values, dtype=np.float32).reshape(-1)
    return dict(
        kernel(
            field,
            str(overlay_type),
            float(center_value),
            None if threshold_mm is None else float(threshold_mm),
            float(max_abs_value),
        )
    )


def nearest_surface_distances(source: MeshDocument, target: MeshDocument) -> np.ndarray:
    kernel = _require_rust_kernel("nearest_surface_distances")
    return np.asarray(kernel(source.vertices, target.vertices, target.faces), dtype=np.float32)


def nearest_vertex_distances(source: MeshDocument, target: MeshDocument) -> np.ndarray:
    kernel = _require_rust_kernel("nearest_vertex_distances")
    return np.asarray(kernel(source.vertices, target.vertices), dtype=np.float32)


def section_contour(
    mesh: MeshDocument,
    *,
    section_constant: float,
    plane_axis: tuple[float, float, float],
    selected_vertex_indices: list[int] | np.ndarray | None = None,
    epsilon: float = 1e-5,
) -> SectionContourPayload:
    kernel = _require_rust_kernel("section_contour")
    selected = (
        None
        if selected_vertex_indices is None
        else np.asarray(selected_vertex_indices, dtype=np.int64).reshape(-1)
    )
    payload: dict[str, Any] = kernel(
        mesh.vertices,
        mesh.faces,
        float(section_constant),
        tuple(float(value) for value in plane_axis),
        selected,
        float(epsilon),
    )
    return SectionContourPayload(
        section_constant=float(payload["section_constant"]),
        plane_axis=tuple(float(value) for value in payload["plane_axis"]),
        plane_u_axis=tuple(float(value) for value in payload["plane_u_axis"]),
        plane_v_axis=tuple(float(value) for value in payload["plane_v_axis"]),
        plane_origin=tuple(float(value) for value in payload["plane_origin"]),
        contour_count=int(payload["contour_count"]),
        segment_count=int(payload["segment_count"]),
        selected_region_segment_count=int(payload["selected_region_segment_count"]),
        perimeter_mm=None if payload["perimeter_mm"] is None else float(payload["perimeter_mm"]),
        width_mm=None if payload["width_mm"] is None else float(payload["width_mm"]),
        depth_mm=None if payload["depth_mm"] is None else float(payload["depth_mm"]),
        projected_bounds_min=None
        if payload["projected_bounds_min"] is None
        else tuple(float(value) for value in payload["projected_bounds_min"]),
        projected_bounds_max=None
        if payload["projected_bounds_max"] is None
        else tuple(float(value) for value in payload["projected_bounds_max"]),
        bounds_min=None if payload["bounds_min"] is None else tuple(float(value) for value in payload["bounds_min"]),
        bounds_max=None if payload["bounds_max"] is None else tuple(float(value) for value in payload["bounds_max"]),
        segments=[
            SectionContourSegment(
                start=tuple(float(value) for value in segment["start"]),
                end=tuple(float(value) for value in segment["end"]),
                selected_region_hit=bool(segment["selected_region_hit"]),
            )
            for segment in payload["segments"]
        ],
    )


def signed_surface_distances(source: MeshDocument, target: MeshDocument) -> np.ndarray:
    kernel = _require_rust_kernel("signed_surface_distances")
    return np.asarray(kernel(source.vertices, target.vertices, target.faces), dtype=np.float32)


def version_compare_distances(source: MeshDocument, target: MeshDocument) -> np.ndarray:
    kernel = _require_rust_kernel("version_compare_distances")
    return np.asarray(kernel(source.vertices, target.vertices, target.faces), dtype=np.float32)


def service_compare_distances(source: MeshDocument, other: MeshDocument) -> np.ndarray:
    kernel = _require_rust_kernel("service_compare_distances")
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


def compare_summary(source: MeshDocument, target: MeshDocument) -> dict[str, float | None]:
    kernel = _require_rust_kernel("compare_summary")
    return _distance_summary_payload(kernel(source.vertices, target.vertices, target.faces), signed=False)


def signed_compare_summary(source: MeshDocument, target: MeshDocument) -> dict[str, float | None]:
    kernel = _require_rust_kernel("signed_compare_summary")
    return _distance_summary_payload(kernel(source.vertices, target.vertices, target.faces), signed=True)


def version_compare_summary(source: MeshDocument, target: MeshDocument) -> VersionCompareSummary:
    kernel = _require_rust_kernel("version_compare_summary")
    payload: dict[str, Any] = kernel(source.vertices, source.faces, target.vertices, target.faces)
    return VersionCompareSummary(
        volume_delta_mm3=float(payload["volume_delta_mm3"]),
        bbox_delta_mm=tuple(float(value) for value in payload["bbox_delta_mm"]),
        min_signed_distance_mm=None if payload["min_signed_distance_mm"] is None else float(payload["min_signed_distance_mm"]),
        max_signed_distance_mm=None if payload["max_signed_distance_mm"] is None else float(payload["max_signed_distance_mm"]),
        mean_signed_distance_mm=None if payload["mean_signed_distance_mm"] is None else float(payload["mean_signed_distance_mm"]),
    )


def service_compare_summary(source: MeshDocument, other: MeshDocument) -> VersionCompareSummary:
    kernel = _require_rust_kernel("service_compare_summary")
    payload: dict[str, Any] = kernel(source.vertices, source.faces, other.vertices, other.faces)
    return VersionCompareSummary(
        volume_delta_mm3=float(payload["volume_delta_mm3"]),
        bbox_delta_mm=tuple(float(value) for value in payload["bbox_delta_mm"]),
        min_signed_distance_mm=None if payload["min_signed_distance_mm"] is None else float(payload["min_signed_distance_mm"]),
        max_signed_distance_mm=None if payload["max_signed_distance_mm"] is None else float(payload["max_signed_distance_mm"]),
        mean_signed_distance_mm=None if payload["mean_signed_distance_mm"] is None else float(payload["mean_signed_distance_mm"]),
    )
