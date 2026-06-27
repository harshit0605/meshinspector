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


def triangles_intersect(triangle_a: Any, triangle_b: Any, *, epsilon: float = 1e-8) -> bool:
    kernel = _require_rust_kernel("triangles_intersect")
    a = np.asarray(triangle_a, dtype=np.float64)
    b = np.asarray(triangle_b, dtype=np.float64)
    return bool(kernel(a, b, float(epsilon)))


def self_intersecting_faces(
    mesh: MeshDocument, *, epsilon: float = 1e-8, touch_is_intersection: bool = True
) -> set[int]:
    kernel = _require_rust_kernel("self_intersecting_faces")
    face_ids = kernel(mesh.vertices, mesh.faces, float(epsilon), bool(touch_is_intersection))
    return {int(face_id) for face_id in face_ids}


def exact_mesh_intersections(
    first: MeshDocument,
    second: MeshDocument,
    *,
    leaf_size: int = 16,
    epsilon: float = 1e-8,
) -> dict[str, np.ndarray]:
    kernel = _require_rust_kernel("exact_mesh_intersections")
    payload = kernel(
        first.vertices,
        first.faces,
        second.vertices,
        second.faces,
        int(max(1, leaf_size)),
        float(epsilon),
    )
    return {
        "first_face_indices": np.asarray(payload["first_face_indices"], dtype=np.int64),
        "second_face_indices": np.asarray(payload["second_face_indices"], dtype=np.int64),
        "intersection_counts": np.asarray(payload["intersection_counts"], dtype=np.int64),
    }
