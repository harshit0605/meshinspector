"""Artifact helpers for current app scalar-field contracts."""

from __future__ import annotations

from pathlib import Path

import numpy as np


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
