from __future__ import annotations

import numpy as np
from pathlib import Path
from typing import Any

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.accelerators._rust_voxel_common import _require_rust_kernel
from geometry_sdk.types import MeshDocument


def voxel_to_mesh_simple_values(
    values: np.ndarray,
    *,
    shape: tuple[int, int, int],
    voxel_size: tuple[float, float, float],
    iso_value: float,
    level_set: bool = False,
) -> dict[str, Any]:
    grid_values = np.asarray(values, dtype=np.float32)
    rust_shape = np.asarray(shape, dtype=np.int64)
    rust_voxel_size = np.asarray(voxel_size, dtype=np.float64)
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if grid_values.size != int(np.prod(rust_shape)):
        raise ValueError("values size must match shape")
    if rust_voxel_size.shape != (3,) or np.any(~np.isfinite(rust_voxel_size)) or np.any(rust_voxel_size <= 0.0):
        raise ValueError("voxel_size must contain three positive finite values")
    if not np.isfinite(iso_value):
        raise ValueError("iso_value must be finite")
    kernel = _require_rust_kernel("voxel_to_mesh_simple_values")
    return dict(
        kernel(
            grid_values.reshape(-1),
            rust_shape,
            rust_voxel_size,
            float(iso_value),
            bool(level_set),
        )
    )


def voxel_to_mesh_dual_values(
    values: np.ndarray,
    *,
    shape: tuple[int, int, int],
    voxel_size: tuple[float, float, float],
    iso_value: float,
    level_set: bool = False,
    adaptivity: float = 0.0,
    max_faces: int | None = None,
    max_vertices: int | None = None,
    relax_disoriented_triangles: bool = True,
) -> dict[str, Any]:
    grid_values = np.asarray(values, dtype=np.float32)
    rust_shape = np.asarray(shape, dtype=np.int64)
    rust_voxel_size = np.asarray(voxel_size, dtype=np.float64)
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if grid_values.size != int(np.prod(rust_shape)):
        raise ValueError("values size must match shape")
    if rust_voxel_size.shape != (3,) or np.any(~np.isfinite(rust_voxel_size)) or np.any(rust_voxel_size <= 0.0):
        raise ValueError("voxel_size must contain three positive finite values")
    if not np.isfinite(iso_value):
        raise ValueError("iso_value must be finite")
    rust_adaptivity = _adaptivity_value(adaptivity)
    rust_max_faces = _mesh_limit_or_unbounded("max_faces", max_faces)
    rust_max_vertices = _mesh_limit_or_unbounded("max_vertices", max_vertices)
    kernel = _require_rust_kernel("voxel_to_mesh_dual_values_with_settings")
    return dict(
        kernel(
            grid_values.reshape(-1),
            rust_shape,
            rust_voxel_size,
            float(iso_value),
            bool(level_set),
            rust_max_faces,
            rust_max_vertices,
            rust_adaptivity,
            bool(relax_disoriented_triangles),
        )
    )

def _mesh_limit_or_unbounded(name: str, value: int | None) -> int:
    if value is None:
        return -1
    if not isinstance(value, int):
        raise ValueError(f"{name} must be an integer")
    if value < 0:
        raise ValueError(f"{name} must be non-negative")
    return value


def _adaptivity_value(value: float) -> float:
    adaptivity = float(value)
    if not np.isfinite(adaptivity) or adaptivity < 0.0 or adaptivity > 1.0:
        raise ValueError("adaptivity must be finite and in [0, 1]")
    return adaptivity


def meshlib_vdb_payload_to_dual_mesh(
    model_bytes: bytes,
    *,
    dimensions: tuple[int, int, int],
    voxel_size: tuple[float, float, float],
    iso_value: float,
    adaptivity: float = 0.0,
    max_faces: int | None = None,
    max_vertices: int | None = None,
    relax_disoriented_triangles: bool = True,
) -> dict[str, Any]:
    if not isinstance(model_bytes, (bytes, bytearray)) or not model_bytes:
        raise ValueError("model_bytes must contain an OpenVDB payload")
    rust_dimensions = np.asarray(dimensions, dtype=np.int64)
    rust_voxel_size = np.asarray(voxel_size, dtype=np.float64)
    if rust_dimensions.shape != (3,) or np.any(rust_dimensions <= 0):
        raise ValueError("dimensions must contain three positive values")
    if rust_voxel_size.shape != (3,) or np.any(~np.isfinite(rust_voxel_size)) or np.any(rust_voxel_size <= 0.0):
        raise ValueError("voxel_size must contain three positive finite values")
    if not np.isfinite(iso_value):
        raise ValueError("iso_value must be finite")
    rust_adaptivity = _adaptivity_value(adaptivity)
    rust_max_faces = _mesh_limit_or_unbounded("max_faces", max_faces)
    rust_max_vertices = _mesh_limit_or_unbounded("max_vertices", max_vertices)
    kernel = _require_rust_kernel("meshlib_vdb_payload_to_dual_mesh")
    return dict(
        kernel(
            bytes(model_bytes),
            rust_dimensions,
            rust_voxel_size,
            float(iso_value),
            rust_max_faces,
            rust_max_vertices,
            rust_adaptivity,
            bool(relax_disoriented_triangles),
        )
    )


def voxel_move_mesh_to_max_deriv_values(
    mesh: MeshDocument,
    values: np.ndarray,
    *,
    shape: tuple[int, int, int],
    voxel_size: tuple[float, float, float],
    iters: int = 30,
    sample_points: int = 6,
    degree: int = 3,
    outlier_threshold: float = 1.0,
    intermediate_smooth_force: float = 0.3,
    preparation_smooth_force: float = 0.1,
    smooth_shift_iterations: int = 15,
    final_relax_iterations: int = 15,
    final_relax_force: float = 0.01,
) -> dict[str, Any]:
    vertices = np.asarray(mesh.vertices, dtype=np.float64)
    faces = np.asarray(mesh.faces, dtype=np.int64)
    grid_values = np.asarray(values, dtype=np.float32)
    rust_shape = np.asarray(shape, dtype=np.int64)
    rust_voxel_size = np.asarray(voxel_size, dtype=np.float64)
    if vertices.ndim != 2 or vertices.shape[1] != 3:
        raise ValueError("mesh vertices must be an Nx3 array")
    if faces.ndim != 2 or faces.shape[1] != 3:
        raise ValueError("mesh faces must be an Nx3 array")
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if grid_values.size != int(np.prod(rust_shape)):
        raise ValueError("values size must match shape")
    if rust_voxel_size.shape != (3,) or np.any(~np.isfinite(rust_voxel_size)) or np.any(rust_voxel_size <= 0.0):
        raise ValueError("voxel_size must contain three positive finite values")
    if sample_points < degree + 1:
        raise ValueError("sample_points must be at least degree + 1")
    kernel = _require_rust_kernel("voxel_move_mesh_to_max_deriv_values")
    return dict(
        kernel(
            vertices,
            faces,
            grid_values.reshape(-1),
            rust_shape,
            rust_voxel_size,
            int(iters),
            int(sample_points),
            int(degree),
            float(outlier_threshold),
            float(intermediate_smooth_force),
            float(preparation_smooth_force),
            int(smooth_shift_iterations),
            int(final_relax_iterations),
            float(final_relax_force),
        )
    )


def voxel_to_mesh_smart_values(
    values: np.ndarray,
    *,
    shape: tuple[int, int, int],
    voxel_size: tuple[float, float, float],
    iso_value: float,
    level_set: bool = False,
    iters: int = 30,
    sample_points: int = 6,
    degree: int = 3,
    outlier_threshold: float = 1.0,
    intermediate_smooth_force: float = 0.3,
    preparation_smooth_force: float = 0.1,
    smooth_shift_iterations: int = 15,
    final_relax_iterations: int = 15,
    final_relax_force: float = 0.01,
) -> dict[str, Any]:
    grid_values = np.asarray(values, dtype=np.float32)
    rust_shape = np.asarray(shape, dtype=np.int64)
    rust_voxel_size = np.asarray(voxel_size, dtype=np.float64)
    if rust_shape.shape != (3,) or np.any(rust_shape <= 0):
        raise ValueError("shape must contain three positive values")
    if grid_values.size != int(np.prod(rust_shape)):
        raise ValueError("values size must match shape")
    if rust_voxel_size.shape != (3,) or np.any(~np.isfinite(rust_voxel_size)) or np.any(rust_voxel_size <= 0.0):
        raise ValueError("voxel_size must contain three positive finite values")
    if not np.isfinite(iso_value):
        raise ValueError("iso_value must be finite")
    if sample_points < degree + 1:
        raise ValueError("sample_points must be at least degree + 1")
    kernel = _require_rust_kernel("voxel_to_mesh_smart_values")
    return dict(
        kernel(
            grid_values.reshape(-1),
            rust_shape,
            rust_voxel_size,
            float(iso_value),
            bool(level_set),
            int(iters),
            int(sample_points),
            int(degree),
            float(outlier_threshold),
            float(intermediate_smooth_force),
            float(preparation_smooth_force),
            int(smooth_shift_iterations),
            int(final_relax_iterations),
            float(final_relax_force),
        )
    )
