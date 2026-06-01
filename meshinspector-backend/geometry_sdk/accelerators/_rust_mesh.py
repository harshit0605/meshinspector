from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument, MeshHealth, MeshStats


def _require_rust_kernel(name: str):
    mode = _common.accelerator_mode()
    if mode == "python":
        return None
    if _common._rs is None:
        if mode == "rust":
            raise RuntimeError("GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs is not installed")
        return None
    if not hasattr(_common._rs, name):
        if mode == "rust":
            raise RuntimeError(f"GEOMETRY_SDK_ACCELERATOR=rust requested, but _zennah_geometry_rs does not expose {name}")
        return None
    return getattr(_common._rs, name)


def _require_core_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def safe_normalize(vectors) -> np.ndarray:
    values = np.asarray(vectors, dtype=np.float64)
    if values.ndim == 1:
        kernel = _require_core_kernel("safe_normalize_vector")
        return np.asarray(kernel(values), dtype=np.float64)
    if values.ndim == 2 and values.shape[1] == 3:
        kernel = _require_core_kernel("safe_normalize_vectors")
        return np.asarray(kernel(values), dtype=np.float64)
    raise ValueError("vectors must have shape (3,) or (n, 3)")


def normalize_axis(axis) -> np.ndarray:
    values = np.asarray(axis, dtype=np.float64)
    kernel = _require_core_kernel("normalize_axis")
    return np.asarray(kernel(values), dtype=np.float64)


def bounds(mesh: MeshDocument) -> tuple[np.ndarray, np.ndarray]:
    kernel = _require_core_kernel("mesh_bounds")
    payload: dict[str, Any] = kernel(mesh.vertices)
    return np.asarray(payload["min"], dtype=np.float64), np.asarray(payload["max"], dtype=np.float64)


def face_normals(mesh: MeshDocument) -> np.ndarray:
    kernel = _require_core_kernel("face_normals")
    return np.asarray(kernel(mesh.vertices, mesh.faces), dtype=np.float64).reshape((-1, 3))


def vertex_normals(mesh: MeshDocument) -> np.ndarray:
    kernel = _require_core_kernel("vertex_normals")
    return np.asarray(kernel(mesh.vertices, mesh.faces), dtype=np.float64).reshape((-1, 3))


def surface_area(mesh: MeshDocument) -> float:
    kernel = _require_core_kernel("surface_area")
    return float(kernel(mesh.vertices, mesh.faces))


def signed_volume(mesh: MeshDocument) -> float:
    kernel = _require_core_kernel("signed_volume")
    return float(kernel(mesh.vertices, mesh.faces))


def volume(mesh: MeshDocument) -> float:
    kernel = _require_core_kernel("volume")
    return float(kernel(mesh.vertices, mesh.faces))


def edge_face_map(mesh: MeshDocument) -> dict[tuple[int, int], list[int]]:
    kernel = _require_core_kernel("edge_face_map")
    payload: dict[tuple[int, int], list[int]] = kernel(mesh.vertices, mesh.faces)
    return {
        (int(edge[0]), int(edge[1])): [int(face_id) for face_id in face_ids]
        for edge, face_ids in payload.items()
    }


def boundary_edges_for_core(mesh: MeshDocument) -> list[tuple[int, int]]:
    kernel = _require_core_kernel("boundary_edges")
    edges = np.asarray(kernel(mesh.vertices, mesh.faces), dtype=np.int64).reshape((-1, 2))
    return [(int(edge[0]), int(edge[1])) for edge in edges]


def face_adjacency(mesh: MeshDocument) -> list[list[int]]:
    kernel = _require_core_kernel("face_adjacency")
    return [[int(face_id) for face_id in neighbors] for neighbors in kernel(mesh.vertices, mesh.faces)]


def connected_face_components(mesh: MeshDocument) -> list[list[int]]:
    kernel = _require_core_kernel("connected_face_components")
    return [[int(face_id) for face_id in component] for component in kernel(mesh.vertices, mesh.faces)]


def vertex_neighbors(mesh: MeshDocument) -> list[list[int]]:
    kernel = _require_core_kernel("vertex_neighbors")
    return [[int(vertex_id) for vertex_id in neighbors] for neighbors in kernel(mesh.vertices, mesh.faces)]


def mesh_stats(mesh: MeshDocument) -> MeshStats | None:
    kernel = _require_rust_kernel("mesh_stats")
    if kernel is None:
        return None

    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces)
    return MeshStats(
        bbox_min=tuple(float(x) for x in payload["bbox_min"]),
        bbox_max=tuple(float(x) for x in payload["bbox_max"]),
        bbox_size=tuple(float(x) for x in payload["bbox_size"]),
        surface_area_mm2=float(payload["surface_area_mm2"]),
        volume_mm3=float(payload["volume_mm3"]),
        vertex_count=int(payload["vertex_count"]),
        face_count=int(payload["face_count"]),
        connected_components=int(payload["connected_components"]),
        boundary_edge_count=int(payload["boundary_edge_count"]),
    )


def boundary_loops(mesh: MeshDocument) -> list[list[int]] | None:
    kernel = _require_rust_kernel("boundary_loops")
    if kernel is None:
        return None
    return [[int(vertex_id) for vertex_id in component] for component in kernel(mesh.vertices, mesh.faces)]


def mesh_health(
    mesh: MeshDocument,
    *,
    detect_self_intersections: bool = True,
    max_self_intersection_faces: int | None = 50000,
    epsilon: float = 1e-8,
) -> MeshHealth | None:
    kernel = _require_rust_kernel("mesh_health")
    if kernel is None:
        return None

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
