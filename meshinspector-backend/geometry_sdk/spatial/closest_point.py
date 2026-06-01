"""Closest-point compatibility wrappers for Rust-owned kernels."""

from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import _rust_closest_point
from geometry_sdk.types import MeshDocument


def closest_point_on_triangle(point: Any, triangle: Any):
    return _rust_closest_point.closest_point_on_triangle(point, triangle)


def closest_points_on_mesh(points: Any, mesh: MeshDocument, *, tree: Any = None):
    _ = tree
    return _rust_closest_point.closest_points_on_mesh(points, mesh)


def point_mesh_distances(points: Any, mesh: MeshDocument, *, tree: Any = None):
    _ = tree
    return _rust_closest_point.point_mesh_distances(points, mesh)
