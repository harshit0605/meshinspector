"""Triangle intersection compatibility wrappers for Rust-owned kernels."""

from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import _rust_intersections
from geometry_sdk.types import MeshDocument


def triangles_intersect(triangle_a: Any, triangle_b: Any, *, epsilon: float = 1e-8) -> bool:
    return _rust_intersections.triangles_intersect(triangle_a, triangle_b, epsilon=epsilon)


def self_intersecting_faces(mesh: MeshDocument, *, epsilon: float = 1e-8, leaf_size: int = 16) -> set[int]:
    _ = leaf_size
    return _rust_intersections.self_intersecting_faces(mesh, epsilon=epsilon)
