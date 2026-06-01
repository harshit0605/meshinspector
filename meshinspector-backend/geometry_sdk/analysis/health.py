"""Mesh health compatibility wrappers for Rust-owned kernels."""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_health
from geometry_sdk.types import MeshDocument, MeshHealth, ServiceMeshHealth


def boundary_loops(mesh: MeshDocument) -> list[list[int]]:
    return _rust_health.boundary_loops(mesh)


def compute_mesh_health(
    mesh: MeshDocument,
    *,
    detect_self_intersections: bool = True,
    max_self_intersection_faces: int | None = 50000,
) -> MeshHealth:
    return _rust_health.mesh_health(
        mesh,
        detect_self_intersections=detect_self_intersections,
        max_self_intersection_faces=max_self_intersection_faces,
    )


def service_mesh_health(
    mesh: MeshDocument,
    *,
    max_listed_faces: int = 100,
    epsilon: float = 1e-8,
) -> ServiceMeshHealth:
    return _rust_health.service_mesh_health(
        mesh,
        max_listed_faces=max_listed_faces,
        epsilon=epsilon,
    )
