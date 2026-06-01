from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def _vec3(name: str, values: Any) -> np.ndarray:
    vector = np.asarray(values, dtype=np.float64)
    if vector.shape != (3,):
        raise ValueError(f"{name} must have shape (3,)")
    return vector


def _optional_distance(max_distance: float) -> float | None:
    distance = float(max_distance)
    return None if distance == float("inf") else distance


def build_aabb_tree(mesh: Any, *, leaf_size: int = 16):
    kernel = _require_rust_kernel("build_aabb_tree")
    return kernel(mesh.vertices, mesh.faces, max(1, int(leaf_size)))


def point_aabb_distance_sq(point: Any, bbox_min: Any, bbox_max: Any) -> float:
    kernel = _require_rust_kernel("point_aabb_distance_sq")
    return float(kernel(_vec3("point", point), _vec3("bbox_min", bbox_min), _vec3("bbox_max", bbox_max)))


def ray_intersects_aabb(
    origin: Any,
    direction: Any,
    bbox_min: Any,
    bbox_max: Any,
    *,
    max_distance: float = float("inf"),
) -> bool:
    kernel = _require_rust_kernel("ray_intersects_aabb")
    return bool(
        kernel(
            _vec3("origin", origin),
            _vec3("direction", direction),
            _vec3("bbox_min", bbox_min),
            _vec3("bbox_max", bbox_max),
            _optional_distance(max_distance),
        )
    )


def ray_candidate_faces(tree: Any, origin: Any, direction: Any, *, max_distance: float = float("inf")) -> np.ndarray:
    kernel = _require_rust_kernel("aabb_ray_candidate_faces")
    return np.asarray(
        kernel(
            tree.rust_tree,
            _vec3("origin", origin),
            _vec3("direction", direction),
            _optional_distance(max_distance),
        ),
        dtype=np.int64,
    )


def overlapping_face_pairs(tree: Any, *, epsilon: float = 0.0) -> list[tuple[int, int]]:
    kernel = _require_rust_kernel("aabb_overlapping_face_pairs")
    return [(int(left), int(right)) for left, right in kernel(tree.rust_tree, float(epsilon))]


def closest_candidate_faces(tree: Any, point: Any, current_best_sq: float) -> np.ndarray:
    kernel = _require_rust_kernel("aabb_closest_candidate_faces")
    return np.asarray(
        kernel(
            tree.rust_tree,
            _vec3("point", point),
            float(current_best_sq),
        ),
        dtype=np.int64,
    )
