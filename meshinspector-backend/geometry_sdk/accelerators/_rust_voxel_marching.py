from __future__ import annotations

import numpy as np
from pathlib import Path
from typing import Any

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.accelerators._rust_voxel_common import _require_rust_kernel
from geometry_sdk.types import MeshDocument


def extract_surface_mesh_from_sdf_cells(
    values: np.ndarray,
    *,
    origin: np.ndarray | tuple[float, float, float],
    shape: tuple[int, int, int],
    voxel_size_mm: float,
    iso_value: float = 0.0,
) -> MeshDocument:
    grid_values = np.asarray(values, dtype=np.float32)
    rust_origin = np.asarray(origin, dtype=np.float64)
    rust_shape = np.asarray(shape, dtype=np.int64)
    if rust_origin.shape != (3,):
        raise ValueError("origin must have shape (3,)")
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if tuple(int(value) for value in rust_shape) != grid_values.shape:
        raise ValueError("values shape must match shape")
    kernel = _require_rust_kernel("extract_surface_mesh_from_sdf_cells")
    payload = kernel(
        grid_values.reshape(-1),
        rust_origin,
        rust_shape,
        float(voxel_size_mm),
        float(iso_value),
    )
    return MeshDocument(
        np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        metadata={"source": "sdf_surface", "voxel_size_mm": float(voxel_size_mm), "iso_value": float(iso_value)},
    )

def sdf_boolean_marching_tetrahedra(
    a_values: np.ndarray,
    b_values: np.ndarray,
    *,
    operation: str,
    origin: np.ndarray | tuple[float, float, float],
    shape: tuple[int, int, int],
    voxel_size_mm: float,
    iso_value: float = 0.0,
) -> tuple[np.ndarray, np.ndarray] | None:
    if operation not in _common.SDF_BOOLEAN_OPERATIONS:
        raise ValueError("operation must be 'union', 'intersection', or 'difference'")
    kernel = _require_rust_kernel("sdf_boolean_marching_tetrahedra")

    left = np.asarray(a_values, dtype=np.float32)
    right = np.asarray(b_values, dtype=np.float32)
    rust_origin = np.asarray(origin, dtype=np.float64)
    rust_shape = np.asarray(shape, dtype=np.int64)
    if left.shape != right.shape:
        raise ValueError("SDF value arrays must have the same shape")
    if rust_origin.shape != (3,):
        raise ValueError("origin must have shape (3,)")
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if tuple(int(value) for value in rust_shape) != left.shape:
        raise ValueError("values shape must match shape")
    if not np.isfinite(voxel_size_mm) or voxel_size_mm <= 0:
        raise ValueError("voxel_size_mm must be positive")

    payload = kernel(
        left.reshape(-1),
        right.reshape(-1),
        operation,
        rust_origin,
        rust_shape,
        float(voxel_size_mm),
        float(iso_value),
    )
    vertices = np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3)
    faces = np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3)
    return vertices, faces

def sdf_offset_marching_tetrahedra(
    values: np.ndarray,
    *,
    origin: np.ndarray | tuple[float, float, float],
    shape: tuple[int, int, int],
    voxel_size_mm: float,
    offset_mm: float,
    iso_value: float = 0.0,
) -> tuple[np.ndarray, np.ndarray]:
    kernel = _require_rust_kernel("sdf_offset_marching_tetrahedra")
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
    if not np.isfinite(offset_mm):
        raise ValueError("offset_mm must be finite")

    payload = kernel(
        grid_values.reshape(-1),
        rust_origin,
        rust_shape,
        float(voxel_size_mm),
        float(offset_mm),
        float(iso_value),
    )
    vertices = np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3)
    faces = np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3)
    return vertices, faces

def sdf_shell_marching_tetrahedra(
    values: np.ndarray,
    *,
    origin: np.ndarray | tuple[float, float, float],
    shape: tuple[int, int, int],
    voxel_size_mm: float,
    wall_thickness_mm: float,
    iso_value: float = 0.0,
) -> tuple[np.ndarray, np.ndarray]:
    kernel = _require_rust_kernel("sdf_shell_marching_tetrahedra")
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
    if not np.isfinite(wall_thickness_mm) or wall_thickness_mm <= 0:
        raise ValueError("wall_thickness_mm must be positive")

    payload = kernel(
        grid_values.reshape(-1),
        rust_origin,
        rust_shape,
        float(voxel_size_mm),
        float(wall_thickness_mm),
        float(iso_value),
    )
    vertices = np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3)
    faces = np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3)
    return vertices, faces

def project_vertices_to_sdf(
    vertices: np.ndarray,
    values: np.ndarray,
    *,
    origin: np.ndarray | tuple[float, float, float],
    shape: tuple[int, int, int],
    voxel_size_mm: float,
    iso_value: float = 0.0,
    iterations: int = 3,
) -> np.ndarray:
    kernel = _require_rust_kernel("project_vertices_to_sdf")
    vertex_array = np.asarray(vertices, dtype=np.float64)
    grid_values = np.asarray(values, dtype=np.float32)
    rust_origin = np.asarray(origin, dtype=np.float64)
    rust_shape = np.asarray(shape, dtype=np.int64)
    if vertex_array.ndim != 2 or vertex_array.shape[1] != 3:
        raise ValueError("vertices must have shape (n, 3)")
    if rust_origin.shape != (3,):
        raise ValueError("origin must have shape (3,)")
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if tuple(int(value) for value in rust_shape) != grid_values.shape:
        raise ValueError("values shape must match shape")
    if not np.isfinite(voxel_size_mm) or voxel_size_mm <= 0:
        raise ValueError("voxel_size_mm must be positive")

    projected = kernel(
        vertex_array,
        grid_values.reshape(-1),
        rust_origin,
        rust_shape,
        float(voxel_size_mm),
        float(iso_value),
        int(iterations),
    )
    return np.asarray(projected, dtype=np.float64).reshape(-1, 3)

def refine_vertices_with_sdf(
    mesh: MeshDocument,
    values: np.ndarray,
    *,
    origin: np.ndarray | tuple[float, float, float],
    shape: tuple[int, int, int],
    voxel_size_mm: float,
    iso_value: float = 0.0,
    smooth_iterations: int = 1,
    smooth_strength: float = 0.2,
    projection_iterations: int = 3,
) -> np.ndarray:
    kernel = _require_rust_kernel("refine_vertices_with_sdf")
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

    refined = kernel(
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

def marching_tetrahedra(
    values: np.ndarray,
    *,
    origin: np.ndarray | tuple[float, float, float],
    shape: tuple[int, int, int],
    voxel_size_mm: float,
    iso_value: float = 0.0,
) -> tuple[np.ndarray, np.ndarray]:
    kernel = _require_rust_kernel("marching_tetrahedra")
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

    payload = kernel(
        grid_values.reshape(-1),
        rust_origin,
        rust_shape,
        float(voxel_size_mm),
        float(iso_value),
    )
    vertices = np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3)
    faces = np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3)
    return vertices, faces
