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


def _mesh_from_payload(payload: dict[str, Any], *, voxel_size_mm: float, iso_value: float) -> MeshDocument:
    return MeshDocument(
        np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        metadata={
            "source": "sdf_marching_tetrahedra",
            "voxel_size_mm": float(voxel_size_mm),
            "iso_value": float(iso_value),
        },
    )


def extract_marching_tetrahedra(
    values: np.ndarray,
    *,
    origin: Any,
    shape: tuple[int, int, int],
    voxel_size_mm: float,
    iso_value: float = 0.0,
) -> MeshDocument:
    grid_values, rust_origin, rust_shape = _grid_inputs(values, origin, shape, voxel_size_mm)
    kernel = _require_rust_kernel("finalized_marching_tetrahedra")
    payload = kernel(
        grid_values.reshape(-1),
        rust_origin,
        rust_shape,
        float(voxel_size_mm),
        float(iso_value),
    )
    return _mesh_from_payload(payload, voxel_size_mm=voxel_size_mm, iso_value=iso_value)


def extract_boolean_marching_tetrahedra(
    a_values: np.ndarray,
    b_values: np.ndarray,
    *,
    operation: str,
    origin: Any,
    shape: tuple[int, int, int],
    voxel_size_mm: float,
    iso_value: float = 0.0,
) -> MeshDocument:
    if operation not in _common.SDF_BOOLEAN_OPERATIONS:
        raise ValueError("operation must be 'union', 'intersection', or 'difference'")
    left, rust_origin, rust_shape = _grid_inputs(a_values, origin, shape, voxel_size_mm)
    right = np.asarray(b_values, dtype=np.float32)
    if right.shape != left.shape:
        raise ValueError("SDF value arrays must have the same shape")
    kernel = _require_rust_kernel("finalized_sdf_boolean_marching_tetrahedra")
    payload = kernel(
        left.reshape(-1),
        right.reshape(-1),
        operation,
        rust_origin,
        rust_shape,
        float(voxel_size_mm),
        float(iso_value),
    )
    return _mesh_from_payload(payload, voxel_size_mm=voxel_size_mm, iso_value=iso_value)


def extract_offset_marching_tetrahedra(
    values: np.ndarray,
    *,
    origin: Any,
    shape: tuple[int, int, int],
    voxel_size_mm: float,
    offset_mm: float,
    iso_value: float = 0.0,
) -> MeshDocument:
    if not np.isfinite(offset_mm):
        raise ValueError("offset_mm must be finite")
    grid_values, rust_origin, rust_shape = _grid_inputs(values, origin, shape, voxel_size_mm)
    kernel = _require_rust_kernel("finalized_sdf_offset_marching_tetrahedra")
    payload = kernel(
        grid_values.reshape(-1),
        rust_origin,
        rust_shape,
        float(voxel_size_mm),
        float(offset_mm),
        float(iso_value),
    )
    return _mesh_from_payload(payload, voxel_size_mm=voxel_size_mm, iso_value=iso_value)


def extract_shell_marching_tetrahedra(
    values: np.ndarray,
    *,
    origin: Any,
    shape: tuple[int, int, int],
    voxel_size_mm: float,
    wall_thickness_mm: float,
    iso_value: float = 0.0,
) -> MeshDocument:
    if not np.isfinite(wall_thickness_mm) or wall_thickness_mm <= 0:
        raise ValueError("wall_thickness_mm must be positive")
    grid_values, rust_origin, rust_shape = _grid_inputs(values, origin, shape, voxel_size_mm)
    kernel = _require_rust_kernel("finalized_sdf_shell_marching_tetrahedra")
    payload = kernel(
        grid_values.reshape(-1),
        rust_origin,
        rust_shape,
        float(voxel_size_mm),
        float(wall_thickness_mm),
        float(iso_value),
    )
    return _mesh_from_payload(payload, voxel_size_mm=voxel_size_mm, iso_value=iso_value)


def orient_faces_consistently(faces: np.ndarray) -> tuple[np.ndarray, list[list[int]]]:
    face_array = np.asarray(faces, dtype=np.int64)
    if face_array.ndim != 2 or face_array.shape[1] != 3:
        raise ValueError("faces must have shape (n, 3)")
    payload = _require_rust_kernel("orient_faces_consistently")(face_array)
    oriented_faces = np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3)
    offsets = np.asarray(payload["component_offsets"], dtype=np.int64).reshape(-1)
    component_faces = np.asarray(payload["component_faces"], dtype=np.int64).reshape(-1)
    components = [
        [int(face_id) for face_id in component_faces[int(start) : int(end)]]
        for start, end in zip(offsets[:-1], offsets[1:])
    ]
    return oriented_faces, components
