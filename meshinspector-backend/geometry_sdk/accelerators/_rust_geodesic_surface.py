from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument


def _require_core_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def mesh_geodesic_distance_field(
    mesh: MeshDocument,
    *,
    seed_vertices,
    max_distance_mm: float | None = None,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_geodesic_distance_field")
    seeds = np.asarray(seed_vertices, dtype=np.int64).reshape((-1,))
    limit = np.finfo(np.float64).max if max_distance_mm is None else float(max_distance_mm)
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, seeds, limit)
    raw_distances = [float(distance) for distance in payload["distances_mm"]]
    raw_predecessors = [int(index) for index in payload["predecessor_vertices"]]
    return {
        "seed_vertices": [int(index) for index in payload["seed_vertices"]],
        "distances_mm": [distance if np.isfinite(distance) else None for distance in raw_distances],
        "predecessor_vertices": [index if index >= 0 else None for index in raw_predecessors],
        "reachable_vertex_count": int(payload["reachable_vertex_count"]),
        "max_distance_mm": float(payload["max_distance_mm"]),
        "meshlib_reference": str(
            payload.get("meshlib_reference", "MR::computeSurfaceDistances / SurfaceDistanceBuilder")
        ),
    }


def mesh_closest_surface_path_targets(
    mesh: MeshDocument,
    *,
    start_vertices,
    end_vertices,
    max_distance_mm: float | None = None,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_closest_surface_path_targets")
    starts = np.asarray(start_vertices, dtype=np.int64).reshape((-1,))
    ends = np.asarray(end_vertices, dtype=np.int64).reshape((-1,))
    limit = np.finfo(np.float64).max if max_distance_mm is None else float(max_distance_mm)
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, starts, ends, limit)
    raw_targets = [int(index) for index in payload["target_vertices"]]
    raw_distances = [float(distance) for distance in payload["distances_mm"]]
    raw_target_distances = [float(distance) for distance in payload["target_distances_mm"]]
    raw_predecessors = [int(index) for index in payload["predecessor_vertices"]]
    return {
        "start_vertices": [int(index) for index in payload["start_vertices"]],
        "end_vertices": [int(index) for index in payload["end_vertices"]],
        "target_vertices": [index if index >= 0 else None for index in raw_targets],
        "target_distances_mm": [distance if np.isfinite(distance) else None for distance in raw_target_distances],
        "distances_mm": [distance if np.isfinite(distance) else None for distance in raw_distances],
        "predecessor_vertices": [index if index >= 0 else None for index in raw_predecessors],
        "meshlib_reference": str(payload.get("meshlib_reference", "MR::computeClosestSurfacePathTargets")),
    }


def mesh_surface_distance_seed_vertices(
    mesh: MeshDocument,
    *,
    seed_vertices=None,
    seed_edges=None,
    seed_face_ids=None,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_surface_distance_seed_vertices")
    seeds = np.asarray([] if seed_vertices is None else seed_vertices, dtype=np.int64).reshape((-1,))
    edges = np.asarray([] if seed_edges is None else seed_edges, dtype=np.int64)
    edges = edges.reshape((0, 2)) if edges.size == 0 else edges.reshape((-1, 2))
    faces = np.asarray([] if seed_face_ids is None else seed_face_ids, dtype=np.int64).reshape((-1,))
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, seeds, edges, faces)
    return {
        "seed_vertices": [int(index) for index in payload["seed_vertices"]],
        "selected_edges": [tuple(int(index) for index in edge) for edge in payload["selected_edges"]],
        "selected_face_indices": [int(index) for index in payload["selected_face_indices"]],
        "selected_face_boundary_edges": [
            tuple(int(index) for index in edge) for edge in payload["selected_face_boundary_edges"]
        ],
        "meshlib_reference": str(
            payload.get("meshlib_reference", "Surface Distance selected edges / selected triangles boundary")
        ),
    }


def mesh_geodesic_iso_region(
    mesh: MeshDocument,
    *,
    seed_vertices,
    iso_value_mm: float,
    max_distance_mm: float | None = None,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_geodesic_iso_region")
    seeds = np.asarray(seed_vertices, dtype=np.int64).reshape((-1,))
    limit = np.finfo(np.float64).max if max_distance_mm is None else float(max_distance_mm)
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, seeds, float(iso_value_mm), limit)
    raw_distances = [float(distance) for distance in payload["distances_mm"]]
    raw_predecessors = [int(index) for index in payload["predecessor_vertices"]]
    return {
        "seed_vertices": [int(index) for index in payload["seed_vertices"]],
        "distances_mm": [distance if np.isfinite(distance) else None for distance in raw_distances],
        "predecessor_vertices": [index if index >= 0 else None for index in raw_predecessors],
        "reachable_vertex_count": int(payload["reachable_vertex_count"]),
        "max_distance_mm": float(payload["max_distance_mm"]),
        "iso_value_mm": float(payload["iso_value_mm"]),
        "selected_vertex_indices": [int(index) for index in payload["selected_vertex_indices"]],
        "selected_face_indices": [int(index) for index in payload["selected_face_indices"]],
        "crossing_face_indices": [int(index) for index in payload["crossing_face_indices"]],
        "boundary_edges": [tuple(int(index) for index in edge) for edge in payload["boundary_edges"]],
        "iso_segments": [
            tuple(tuple(float(coordinate) for coordinate in point) for point in segment)
            for segment in payload["iso_segments"]
        ],
        "clipped_vertices": [tuple(float(coordinate) for coordinate in point) for point in payload["clipped_vertices"]],
        "clipped_faces": [tuple(int(index) for index in face) for face in payload["clipped_faces"]],
        "clipped_source_face_indices": [int(index) for index in payload["clipped_source_face_indices"]],
        "clipped_source_vertex_indices": [
            index if index >= 0 else None for index in (int(value) for value in payload["clipped_source_vertex_indices"])
        ],
        "meshlib_reference": str(
            payload.get("meshlib_reference", "MR::computeClosestSurfacePathTargets surface-distance iso")
        ),
    }
