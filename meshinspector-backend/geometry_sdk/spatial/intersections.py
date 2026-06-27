"""Triangle intersection compatibility wrappers for Rust-owned kernels."""

from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import _rust_intersections
from geometry_sdk.types import MeshCollisionFacePair, MeshCollisionResult, MeshDocument


def triangles_intersect(triangle_a: Any, triangle_b: Any, *, epsilon: float = 1e-8) -> bool:
    return _rust_intersections.triangles_intersect(triangle_a, triangle_b, epsilon=epsilon)


def exact_mesh_intersections(
    first: MeshDocument,
    second: MeshDocument,
    *,
    leaf_size: int = 16,
    epsilon: float = 1e-8,
    first_intersection_only: bool = False,
    max_pairs: int | None = None,
) -> MeshCollisionResult:
    payload = _rust_intersections.exact_mesh_intersections(
        first,
        second,
        leaf_size=leaf_size,
        epsilon=epsilon,
    )
    if payload is None:
        raise RuntimeError("Rust exact_mesh_intersections kernel is required for collision detection")

    first_indices = [int(value) for value in payload["first_face_indices"].reshape(-1).tolist()]
    second_indices = [int(value) for value in payload["second_face_indices"].reshape(-1).tolist()]
    counts = [int(value) for value in payload["intersection_counts"].reshape(-1).tolist()]
    raw_pairs = [
        MeshCollisionFacePair(first_face=first_face, second_face=second_face, intersection_count=count)
        for first_face, second_face, count in zip(first_indices, second_indices, counts, strict=True)
    ]
    limit = 1 if first_intersection_only else max_pairs
    truncated = limit is not None and len(raw_pairs) > max(0, int(limit))
    pairs = raw_pairs[: max(0, int(limit))] if limit is not None else raw_pairs

    return MeshCollisionResult(
        colliding=bool(pairs),
        pair_count=len(pairs),
        first_face_indices=sorted({pair.first_face for pair in pairs}),
        second_face_indices=sorted({pair.second_face for pair in pairs}),
        pairs=pairs,
        truncated=truncated,
        metadata={
            "raw_pair_count": len(raw_pairs),
            "meshlib_reference": "findCollidingTriangles",
            "rust_backed": True,
        },
    )


def self_intersecting_faces(
    mesh: MeshDocument,
    *,
    epsilon: float = 1e-8,
    leaf_size: int = 16,
    touch_is_intersection: bool = True,
) -> set[int]:
    _ = leaf_size
    return _rust_intersections.self_intersecting_faces(
        mesh, epsilon=epsilon, touch_is_intersection=touch_is_intersection
    )
