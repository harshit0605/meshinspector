from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument, MeshHealth, ServiceMeshHealth


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def boundary_loops(mesh: MeshDocument) -> list[list[int]]:
    kernel = _require_rust_kernel("boundary_loops")
    return [[int(vertex_id) for vertex_id in component] for component in kernel(mesh.vertices, mesh.faces)]


def mesh_health(
    mesh: MeshDocument,
    *,
    detect_self_intersections: bool = True,
    max_self_intersection_faces: int | None = 50000,
    epsilon: float = 1e-8,
) -> MeshHealth:
    kernel = _require_rust_kernel("mesh_health")
    payload: dict[str, Any] = kernel(
        mesh.vertices,
        mesh.faces,
        bool(detect_self_intersections),
        max_self_intersection_faces,
        float(epsilon),
    )
    return MeshHealth(
        is_closed=bool(payload["is_closed"]),
        holes_count=int(payload["holes_count"]),
        boundary_edge_count=int(payload["boundary_edge_count"]),
        nonmanifold_edge_count=int(payload["nonmanifold_edge_count"]),
        self_intersections=None if payload["self_intersections"] is None else int(payload["self_intersections"]),
        self_intersections_available=bool(payload["self_intersections_available"]),
    )


def service_mesh_health(
    mesh: MeshDocument,
    *,
    max_listed_faces: int = 100,
    epsilon: float = 1e-8,
) -> ServiceMeshHealth:
    kernel = _require_rust_kernel("service_mesh_health")
    payload: dict[str, Any] = kernel(
        mesh.vertices,
        mesh.faces,
        int(max_listed_faces),
        float(epsilon),
    )
    return ServiceMeshHealth(
        is_closed=bool(payload["is_closed"]),
        self_intersections=int(payload["self_intersections"]),
        self_intersection_faces=[int(face) for face in payload["self_intersection_faces"]],
        holes_count=int(payload["holes_count"]),
        degenerate_faces=int(payload["degenerate_faces"]),
        health_score=int(payload["health_score"]),
    )
