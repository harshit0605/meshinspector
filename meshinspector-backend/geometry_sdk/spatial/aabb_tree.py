"""AABB tree compatibility wrappers for Rust-owned spatial broad-phase kernels."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from geometry_sdk.accelerators import _rust_aabb_tree
from geometry_sdk.types import MeshDocument


@dataclass(slots=True)
class AABBNode:
    bbox_min: Any | None = None
    bbox_max: Any | None = None
    face_indices: Any | None = None
    face_count: int = 0
    left: "AABBNode | None" = None
    right: "AABBNode | None" = None

    @property
    def is_leaf(self) -> bool:
        return self.left is None and self.right is None

    @property
    def subtree_face_count(self) -> int:
        return int(self.face_count)


@dataclass(slots=True)
class AABBTree:
    mesh: MeshDocument
    leaf_size: int = 16
    root: AABBNode | None = None
    rust_tree: Any | None = None

    @property
    def triangles(self):
        return self.mesh.vertices[self.mesh.faces]


def build_aabb_tree(mesh: MeshDocument, *, leaf_size: int = 16) -> AABBTree:
    clamped_leaf_size = max(1, int(leaf_size))
    root = None if mesh.face_count == 0 else AABBNode(face_count=mesh.face_count)
    rust_tree = _rust_aabb_tree.build_aabb_tree(mesh, leaf_size=clamped_leaf_size)
    return AABBTree(mesh=mesh, leaf_size=clamped_leaf_size, root=root, rust_tree=rust_tree)


def point_aabb_distance_sq(point: Any, bbox_min: Any, bbox_max: Any) -> float:
    return _rust_aabb_tree.point_aabb_distance_sq(point, bbox_min, bbox_max)


def ray_intersects_aabb(
    origin: Any,
    direction: Any,
    bbox_min: Any,
    bbox_max: Any,
    *,
    max_distance: float = float("inf"),
) -> bool:
    return _rust_aabb_tree.ray_intersects_aabb(
        origin,
        direction,
        bbox_min,
        bbox_max,
        max_distance=max_distance,
    )


def ray_candidate_faces(
    tree: AABBTree,
    origin: Any,
    direction: Any,
    *,
    max_distance: float = float("inf"),
):
    return _rust_aabb_tree.ray_candidate_faces(tree, origin, direction, max_distance=max_distance)


def overlapping_face_pairs(tree: AABBTree, *, epsilon: float = 0.0) -> list[tuple[int, int]]:
    return _rust_aabb_tree.overlapping_face_pairs(tree, epsilon=epsilon)


def closest_candidate_faces(tree: AABBTree, point: Any, current_best_sq: float):
    return _rust_aabb_tree.closest_candidate_faces(tree, point, current_best_sq)
