from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def _optional_vec3(value: Any) -> np.ndarray | None:
    if value is None:
        return None
    return np.asarray(value, dtype=np.float64)


def _optional_indices(value: Any) -> np.ndarray | None:
    if value is None:
        return None
    return np.asarray(value, dtype=np.int64).reshape(-1)


def radial_scale_vertices(
    mesh: MeshDocument,
    scale_factor: float,
    *,
    ring_axis: Any = None,
    preserve_indices: Any = None,
) -> np.ndarray | None:
    kernel = _require_rust_kernel("radial_scale_vertices")
    scaled = kernel(
        mesh.vertices,
        float(scale_factor),
        _optional_vec3(ring_axis),
        _optional_indices(preserve_indices),
    )
    return np.asarray(scaled, dtype=np.float64).reshape(-1, 3)


def resize_ring_vertices(
    mesh: MeshDocument,
    current_size: float,
    target_size: float,
    *,
    ring_axis: Any = None,
    preserve_indices: Any = None,
) -> np.ndarray | None:
    kernel = _require_rust_kernel("resize_ring_vertices")
    scaled = kernel(
        mesh.vertices,
        float(current_size),
        float(target_size),
        _optional_vec3(ring_axis),
        _optional_indices(preserve_indices),
    )
    return np.asarray(scaled, dtype=np.float64).reshape(-1, 3)


def fit_ring_to_diameter_vertices(
    mesh: MeshDocument,
    measured_diameter_mm: float,
    target_diameter_mm: float,
    *,
    ring_axis: Any = None,
    preserve_indices: Any = None,
    max_preserve_scale_ratio: float = 1.5,
) -> tuple[np.ndarray, bool, float]:
    kernel = _require_rust_kernel("fit_ring_to_diameter_vertices")
    scaled, applied_uniform_fallback, scale_factor = kernel(
        mesh.vertices,
        float(measured_diameter_mm),
        float(target_diameter_mm),
        _optional_vec3(ring_axis),
        _optional_indices(preserve_indices),
        float(max_preserve_scale_ratio),
    )
    return (
        np.asarray(scaled, dtype=np.float64).reshape(-1, 3),
        bool(applied_uniform_fallback),
        float(scale_factor),
    )
