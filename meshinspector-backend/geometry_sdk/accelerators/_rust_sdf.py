from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument, SDFGrid


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def sample_sdf_grid(mesh: MeshDocument, *, voxel_size_mm: float, padding_mm: float | None = None) -> SDFGrid:
    if voxel_size_mm <= 0:
        raise ValueError("voxel_size_mm must be positive")
    bounds = _require_rust_kernel("mesh_bounds")(mesh.vertices)
    padding = float(voxel_size_mm if padding_mm is None else padding_mm)
    return sample_sdf_grid_in_bounds(
        mesh,
        bbox_min=np.asarray(bounds["min"], dtype=np.float64),
        bbox_max=np.asarray(bounds["max"], dtype=np.float64),
        voxel_size_mm=voxel_size_mm,
        padding_mm=padding,
    )


def sample_sdf_grid_in_bounds(
    mesh: MeshDocument,
    *,
    bbox_min: Any,
    bbox_max: Any,
    voxel_size_mm: float,
    padding_mm: float = 0.0,
    origin_phase: tuple[float, float, float] | None = None,
) -> SDFGrid:
    if voxel_size_mm <= 0:
        raise ValueError("voxel_size_mm must be positive")
    phase = np.zeros(3, dtype=np.float64) if origin_phase is None else np.asarray(origin_phase, dtype=np.float64)
    if phase.shape != (3,):
        raise ValueError("origin_phase must contain three values")
    sample = _require_rust_kernel("sample_sdf_grid_in_bounds")(
        mesh.vertices,
        mesh.faces,
        np.asarray(bbox_min, dtype=np.float64),
        np.asarray(bbox_max, dtype=np.float64),
        float(voxel_size_mm),
        float(padding_mm),
        phase,
        0.5,
    )
    shape_values = sample["shape"]
    origin_values = sample["origin"]
    shape = (int(shape_values[0]), int(shape_values[1]), int(shape_values[2]))
    return SDFGrid(
        origin=(float(origin_values[0]), float(origin_values[1]), float(origin_values[2])),
        voxel_size_mm=float(voxel_size_mm),
        shape=shape,
        values=np.asarray(sample["values"], dtype=np.float32).reshape(shape),
    )


def sample_aligned_sdf_grids(
    meshes: list[MeshDocument],
    *,
    voxel_size_mm: float,
    padding_mm: float | None = None,
    origin_phase: tuple[float, float, float] | None = None,
) -> list[SDFGrid]:
    if not meshes:
        return []
    if voxel_size_mm <= 0:
        raise ValueError("voxel_size_mm must be positive")
    bounds = [_require_rust_kernel("mesh_bounds")(mesh.vertices) for mesh in meshes]
    combined = _require_rust_kernel("combine_bounding_boxes")(
        np.asarray([entry["min"] for entry in bounds], dtype=np.float64),
        np.asarray([entry["max"] for entry in bounds], dtype=np.float64),
    )
    bbox_min = np.asarray(combined["min"], dtype=np.float64)
    bbox_max = np.asarray(combined["max"], dtype=np.float64)
    padding = float(voxel_size_mm if padding_mm is None else padding_mm)
    return [
        sample_sdf_grid_in_bounds(
            mesh,
            bbox_min=bbox_min,
            bbox_max=bbox_max,
            voxel_size_mm=voxel_size_mm,
            padding_mm=padding,
            origin_phase=origin_phase,
        )
        for mesh in meshes
    ]


def sdf_cell_values(grid: SDFGrid) -> np.ndarray:
    cell_shape = tuple(max(int(dimension) - 1, 0) for dimension in grid.shape)
    if min(grid.shape) < 2:
        return np.zeros(cell_shape, dtype=np.float32)
    output = _require_rust_kernel("sdf_cell_values")(
        np.asarray(grid.values, dtype=np.float32).reshape(-1),
        np.asarray(grid.shape, dtype=np.int64),
    )
    return np.asarray(output, dtype=np.float32).reshape(cell_shape)


def sdf_occupancy(grid: SDFGrid, *, iso_value: float = 0.0) -> np.ndarray:
    cell_shape = tuple(max(int(dimension) - 1, 0) for dimension in grid.shape)
    if min(grid.shape) < 2:
        return np.zeros(cell_shape, dtype=bool)
    output = _require_rust_kernel("sdf_occupancy")(
        np.asarray(grid.values, dtype=np.float32).reshape(-1),
        np.asarray(grid.shape, dtype=np.int64),
        float(iso_value),
    )
    return np.asarray(output, dtype=np.uint8).reshape(cell_shape).astype(bool, copy=False)


def estimate_sdf_volume(grid: SDFGrid, *, iso_value: float = 0.0) -> float:
    return float(
        _require_rust_kernel("estimate_sdf_volume")(
            np.asarray(grid.values, dtype=np.float32).reshape(-1),
            np.asarray(grid.shape, dtype=np.int64),
            float(grid.voxel_size_mm),
            float(iso_value),
        )
    )


def sdf_grid_points(grid: SDFGrid) -> np.ndarray:
    output = _require_rust_kernel("sdf_grid_points")(
        np.asarray(grid.origin, dtype=np.float64),
        np.asarray(grid.shape, dtype=np.int64),
        float(grid.voxel_size_mm),
    )
    return np.asarray(output, dtype=np.float64).reshape(-1, 3)


def sdf_points_to_grid(grid: SDFGrid, points: Any) -> np.ndarray:
    query = np.asarray(points, dtype=np.float64)
    if query.ndim == 1:
        query = query.reshape(1, 3)
    output = _require_rust_kernel("sdf_points_to_grid")(
        np.asarray(grid.origin, dtype=np.float64),
        float(grid.voxel_size_mm),
        query,
    )
    return np.asarray(output, dtype=np.float64).reshape(-1, 3)


def sample_sdf_values(grid: SDFGrid, points: Any) -> np.ndarray:
    query = np.asarray(points, dtype=np.float64)
    if query.ndim == 1:
        query = query.reshape(1, 3)
    output = _require_rust_kernel("sample_sdf_values")(
        np.asarray(grid.values, dtype=np.float32).reshape(-1),
        np.asarray(grid.origin, dtype=np.float64),
        np.asarray(grid.shape, dtype=np.int64),
        float(grid.voxel_size_mm),
        query,
    )
    return np.asarray(output, dtype=np.float32).reshape(-1)


def sample_sdf_gradients(grid: SDFGrid, points: Any) -> np.ndarray:
    query = np.asarray(points, dtype=np.float64)
    if query.ndim == 1:
        query = query.reshape(1, 3)
    output = _require_rust_kernel("sample_sdf_gradients")(
        np.asarray(grid.values, dtype=np.float32).reshape(-1),
        np.asarray(grid.origin, dtype=np.float64),
        np.asarray(grid.shape, dtype=np.int64),
        float(grid.voxel_size_mm),
        query,
    )
    return np.asarray(output, dtype=np.float32).reshape(-1, 3)
