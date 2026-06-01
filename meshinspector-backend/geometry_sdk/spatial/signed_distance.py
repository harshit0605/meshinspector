"""Signed-distance compatibility wrappers for Rust-owned kernels."""

from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import _rust_signed_distance
from geometry_sdk.types import MeshDocument


DEFAULT_WINDING_SIGN_SELF_INTERSECTION_FACE_LIMIT = 50000


def supports_winding_sign(
    mesh: MeshDocument,
    *,
    reject_self_intersections: bool = True,
    max_self_intersection_faces: int | None = DEFAULT_WINDING_SIGN_SELF_INTERSECTION_FACE_LIMIT,
) -> bool:
    return _rust_signed_distance.supports_winding_sign(
        mesh,
        reject_self_intersections=reject_self_intersections,
        max_self_intersection_faces=max_self_intersection_faces,
    )


def point_inside_mesh(
    mesh: MeshDocument,
    point: Any,
    *,
    direction: tuple[float, float, float] = (1.0, 0.371, 0.219),
    epsilon: float = 1e-7,
    tree: Any = None,
) -> bool:
    _ = tree
    return _rust_signed_distance.point_inside_mesh(mesh, point, direction=direction, epsilon=epsilon)


def winding_numbers(points: Any, mesh: MeshDocument, *, chunk_size: int = 2048):
    _ = chunk_size
    return _rust_signed_distance.winding_numbers(points, mesh)


def point_inside_mesh_winding(
    mesh: MeshDocument,
    point: Any,
    *,
    threshold: float = 0.5,
    require_closed: bool = True,
) -> bool:
    return _rust_signed_distance.point_inside_mesh_winding(
        mesh,
        point,
        threshold=threshold,
        require_closed=require_closed,
    )


def signed_point_mesh_distances(
    points: Any,
    mesh: MeshDocument,
    *,
    tree: Any = None,
    sign_method: str = "auto",
):
    _ = tree
    return _rust_signed_distance.signed_point_mesh_distances(points, mesh, sign_method=sign_method)
