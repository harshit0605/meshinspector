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


def closest_point_on_triangle(point: Any, triangle: Any) -> np.ndarray:
    kernel = _require_rust_kernel("closest_point_on_triangle")
    return np.asarray(
        kernel(
            np.asarray(point, dtype=np.float64),
            np.asarray(triangle, dtype=np.float64),
        ),
        dtype=np.float64,
    )


def closest_points_on_mesh(points: Any, mesh: MeshDocument) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    kernel = _require_rust_kernel("closest_points_on_mesh")
    query = np.asarray(points, dtype=np.float64)
    if query.ndim == 1:
        query = query.reshape(1, 3)
    payload = kernel(query, mesh.vertices, mesh.faces)
    return (
        np.asarray(payload["closest_points"], dtype=np.float64).reshape(-1, 3),
        np.asarray(payload["distances"], dtype=np.float64),
        np.asarray(payload["face_indices"], dtype=np.int64),
    )


def point_mesh_distances(points: Any, mesh: MeshDocument) -> np.ndarray:
    kernel = _require_rust_kernel("point_mesh_distances")
    query = np.asarray(points, dtype=np.float64)
    if query.ndim == 1:
        query = query.reshape(1, 3)
    return np.asarray(kernel(query, mesh.vertices, mesh.faces), dtype=np.float32)
