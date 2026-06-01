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


def _ignored_faces(ignore_faces: set[int] | np.ndarray | None) -> np.ndarray:
    if ignore_faces is None:
        return np.zeros(0, dtype=np.int64)
    values = list(ignore_faces) if isinstance(ignore_faces, set) else ignore_faces
    return np.asarray(values, dtype=np.int64).reshape(-1)


def ray_triangle_hits(
    mesh: MeshDocument,
    origin: Any,
    direction: Any,
    *,
    epsilon: float = 1e-8,
    ignore_faces: set[int] | np.ndarray | None = None,
) -> dict[str, np.ndarray]:
    kernel = _require_rust_kernel("ray_triangle_hits")
    payload = kernel(
        mesh.vertices,
        mesh.faces,
        np.asarray(origin, dtype=np.float64),
        np.asarray(direction, dtype=np.float64),
        float(epsilon),
        _ignored_faces(ignore_faces),
    )
    return {
        "face_indices": np.asarray(payload["face_indices"], dtype=np.int64),
        "distances": np.asarray(payload["distances"], dtype=np.float64),
        "points": np.asarray(payload["points"], dtype=np.float64).reshape(-1, 3),
    }


def first_ray_hit(
    mesh: MeshDocument,
    origin: Any,
    direction: Any,
    *,
    epsilon: float = 1e-8,
    ignore_faces: set[int] | np.ndarray | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("first_ray_hit")
    payload = kernel(
        mesh.vertices,
        mesh.faces,
        np.asarray(origin, dtype=np.float64),
        np.asarray(direction, dtype=np.float64),
        float(epsilon),
        _ignored_faces(ignore_faces),
    )
    if payload is None:
        return None
    return {
        "face_index": int(payload["face_index"]),
        "distance": float(payload["distance"]),
        "point": tuple(float(value) for value in payload["point"]),
    }


def first_ray_hits(
    mesh: MeshDocument,
    origins: Any,
    directions: Any,
    *,
    epsilon: float = 1e-8,
    ignore_faces: set[int] | np.ndarray | None = None,
) -> dict[str, np.ndarray]:
    kernel = _require_rust_kernel("first_ray_hits")
    payload = kernel(
        mesh.vertices,
        mesh.faces,
        np.asarray(origins, dtype=np.float64),
        np.asarray(directions, dtype=np.float64),
        float(epsilon),
        _ignored_faces(ignore_faces),
    )
    return {
        "face_indices": np.asarray(payload["face_indices"], dtype=np.int64),
        "distances": np.asarray(payload["distances"], dtype=np.float64),
        "points": np.asarray(payload["points"], dtype=np.float64).reshape(-1, 3),
    }
