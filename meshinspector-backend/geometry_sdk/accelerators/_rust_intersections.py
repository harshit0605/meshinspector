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


def self_intersecting_faces(mesh: MeshDocument, *, epsilon: float = 1e-8) -> set[int]:
    kernel = _require_rust_kernel("self_intersecting_faces")
    face_ids = kernel(mesh.vertices, mesh.faces, float(epsilon))
    return {int(face_id) for face_id in face_ids}
