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


def supports_winding_sign(
    mesh: MeshDocument,
    *,
    reject_self_intersections: bool = True,
    max_self_intersection_faces: int | None = 50000,
    epsilon: float = 1e-8,
) -> bool:
    kernel = _require_rust_kernel("supports_winding_sign")
    return bool(
        kernel(
            mesh.vertices,
            mesh.faces,
            bool(reject_self_intersections),
            max_self_intersection_faces,
            float(epsilon),
        )
    )


def point_inside_mesh(
    mesh: MeshDocument,
    point: Any,
    *,
    direction: tuple[float, float, float] = (1.0, 0.371, 0.219),
    epsilon: float = 1e-7,
) -> bool:
    kernel = _require_rust_kernel("point_inside_mesh")
    return bool(
        kernel(
            mesh.vertices,
            mesh.faces,
            np.asarray(point, dtype=np.float64),
            np.asarray(direction, dtype=np.float64),
            float(epsilon),
        )
    )


def point_inside_mesh_winding(
    mesh: MeshDocument,
    point: Any,
    *,
    threshold: float = 0.5,
    require_closed: bool = True,
) -> bool:
    kernel = _require_rust_kernel("point_inside_mesh_winding")
    return bool(
        kernel(
            mesh.vertices,
            mesh.faces,
            np.asarray(point, dtype=np.float64),
            float(threshold),
            bool(require_closed),
        )
    )


def winding_numbers(points: Any, mesh: MeshDocument) -> np.ndarray:
    kernel = _require_rust_kernel("winding_numbers")
    query = np.asarray(points, dtype=np.float64)
    if query.ndim == 1:
        query = query.reshape(1, 3)
    return np.asarray(kernel(query, mesh.vertices, mesh.faces), dtype=np.float64)


def signed_point_mesh_distances(points: Any, mesh: MeshDocument, *, sign_method: str = "auto") -> np.ndarray:
    kernel = _require_rust_kernel("signed_point_mesh_distances_with_method")
    query = np.asarray(points, dtype=np.float64)
    if query.ndim == 1:
        query = query.reshape(1, 3)
    return np.asarray(kernel(query, mesh.vertices, mesh.faces, sign_method), dtype=np.float32)
