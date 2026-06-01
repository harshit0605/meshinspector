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


def _grid_inputs(values: np.ndarray, origin: Any, shape: tuple[int, int, int], voxel_size_mm: float):
    grid_values = np.asarray(values, dtype=np.float32)
    rust_origin = np.asarray(origin, dtype=np.float64)
    rust_shape = np.asarray(shape, dtype=np.int64)
    if rust_origin.shape != (3,):
        raise ValueError("origin must have shape (3,)")
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if tuple(int(value) for value in rust_shape) != grid_values.shape:
        raise ValueError("values shape must match shape")
    if not np.isfinite(voxel_size_mm) or voxel_size_mm <= 0:
        raise ValueError("voxel_size_mm must be positive")
    return grid_values, rust_origin, rust_shape


def project_vertices_to_sdf(
    mesh: MeshDocument,
    values: np.ndarray,
    *,
    origin: Any,
    shape: tuple[int, int, int],
    voxel_size_mm: float,
    iso_value: float = 0.0,
    iterations: int = 3,
) -> np.ndarray:
    grid_values, rust_origin, rust_shape = _grid_inputs(values, origin, shape, voxel_size_mm)
    projected = _require_rust_kernel("project_vertices_to_sdf")(
        mesh.vertices,
        grid_values.reshape(-1),
        rust_origin,
        rust_shape,
        float(voxel_size_mm),
        float(iso_value),
        int(iterations),
    )
    return np.asarray(projected, dtype=np.float64).reshape(-1, 3)


def laplacian_smooth_vertices(
    mesh: MeshDocument,
    *,
    iterations: int = 1,
    strength: float = 0.25,
) -> np.ndarray:
    smoothed = _require_rust_kernel("laplacian_smooth_vertices")(
        mesh.vertices,
        mesh.faces,
        int(iterations),
        float(np.clip(strength, 0.0, 1.0)),
    )
    return np.asarray(smoothed, dtype=np.float64).reshape(-1, 3)


def refine_vertices_with_sdf(
    mesh: MeshDocument,
    values: np.ndarray,
    *,
    origin: Any,
    shape: tuple[int, int, int],
    voxel_size_mm: float,
    iso_value: float = 0.0,
    smooth_iterations: int = 1,
    smooth_strength: float = 0.2,
    projection_iterations: int = 3,
) -> np.ndarray:
    grid_values, rust_origin, rust_shape = _grid_inputs(values, origin, shape, voxel_size_mm)
    refined = _require_rust_kernel("refine_vertices_with_sdf")(
        mesh.vertices,
        mesh.faces,
        grid_values.reshape(-1),
        rust_origin,
        rust_shape,
        float(voxel_size_mm),
        float(iso_value),
        int(smooth_iterations),
        float(np.clip(smooth_strength, 0.0, 1.0)),
        int(projection_iterations),
    )
    return np.asarray(refined, dtype=np.float64).reshape(-1, 3)
