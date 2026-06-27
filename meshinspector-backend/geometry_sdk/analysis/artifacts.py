"""Artifact helpers for current app scalar-field contracts."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_analysis

SCALAR_OVERLAY_MAX_ABS_VALUE = 1_000_000.0


def validate_vertex_scalar_field(values: np.ndarray, *, vertex_count: int, field_name: str = "values") -> np.ndarray:
    scalars = np.asarray(values, dtype=np.float32)
    if scalars.ndim != 1:
        raise ValueError(f"{field_name} must be a 1D scalar field")
    if scalars.shape[0] != int(vertex_count):
        raise ValueError(f"{field_name} length {scalars.shape[0]} does not match vertex count {vertex_count}")
    return scalars


def save_thickness_npz(
    path: str | Path,
    thickness: np.ndarray,
    *,
    vertex_count: int,
    threshold_mm: float,
) -> Path:
    field = validate_vertex_scalar_field(thickness, vertex_count=vertex_count, field_name="thickness")
    output = Path(path)
    np.savez_compressed(output, thickness=field, threshold_mm=np.float32(threshold_mm))
    return output


def load_thickness_npz(path: str | Path) -> tuple[np.ndarray, float]:
    payload = np.load(Path(path))
    return payload["thickness"].astype(np.float32), float(payload["threshold_mm"])


def _scalar_overlay_payload(
    values: np.ndarray,
    *,
    overlay_type: str,
    center_value: float,
    threshold_mm: float | None,
    summary: dict[str, Any] | None = None,
) -> dict[str, Any]:
    rust_payload = _rust_analysis.scalar_overlay_payload(
        values,
        overlay_type=overlay_type,
        center_value=center_value,
        threshold_mm=threshold_mm,
        max_abs_value=SCALAR_OVERLAY_MAX_ABS_VALUE,
    )
    payload: dict[str, Any] = {
        "overlay_type": rust_payload["overlay_type"],
        "values": rust_payload["values"],
        "min_value": rust_payload["min_value"],
        "max_value": rust_payload["max_value"],
        "center_value": rust_payload["center_value"],
        "threshold_mm": rust_payload["threshold_mm"],
    }
    if summary is not None:
        payload["summary"] = summary
    return payload


def thickness_overlay_payload(path: str | Path) -> dict[str, Any]:
    values, threshold = load_thickness_npz(path)
    return _scalar_overlay_payload(
        values,
        overlay_type="thickness",
        center_value=threshold,
        threshold_mm=threshold,
    )


def save_compare_npz(
    path: str | Path,
    values: np.ndarray,
    *,
    vertex_count: int,
    other_version_id: str,
) -> Path:
    field = validate_vertex_scalar_field(values, vertex_count=vertex_count, field_name="values")
    output = Path(path)
    np.savez_compressed(output, values=np.nan_to_num(field, nan=0.0), other_version_id=np.array([other_version_id]))
    return output


def load_compare_npz(path: str | Path) -> tuple[np.ndarray, str]:
    payload = np.load(Path(path))
    values = payload["values"].astype(np.float32)
    other_version_id = str(payload["other_version_id"][0])
    return values, other_version_id


def compare_overlay_payload(path: str | Path, *, other_version_id: str | None = None) -> dict[str, Any]:
    values, stored_other_version_id = load_compare_npz(path)
    resolved_other_version_id = other_version_id or stored_other_version_id
    overlay = _rust_analysis.scalar_overlay_payload(
        values,
        overlay_type="compare",
        center_value=0.0,
        threshold_mm=None,
        max_abs_value=SCALAR_OVERLAY_MAX_ABS_VALUE,
    )
    return _scalar_overlay_payload(
        values,
        overlay_type="compare",
        center_value=0.0,
        threshold_mm=None,
        summary={
            "other_version_id": resolved_other_version_id,
            "max_abs_distance_mm": overlay["max_abs_value"],
            "mean_distance_mm": overlay["mean_value"],
            "cached": True,
        },
    )
