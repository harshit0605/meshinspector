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


def mesh_fast_marching_surface_path(
    mesh: MeshDocument,
    *,
    start_vertex: int,
    end_vertex: int,
    max_steps: int = 1024,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_fast_marching_surface_path")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, int(start_vertex), int(end_vertex), int(max_steps))
    raw_predecessors = [int(index) for index in payload["surface_predecessor_vertices"]]
    raw_distances = [float(distance) for distance in payload["surface_distances_mm"]]
    return {
        "start_vertex": int(payload["start_vertex"]),
        "end_vertex": int(payload["end_vertex"]),
        "start_face_index": int(payload["start_face_index"]),
        "start_barycentric": tuple(float(value) for value in payload["start_barycentric"]),
        "surface_distances_mm": [distance if np.isfinite(distance) else None for distance in raw_distances],
        "surface_predecessor_vertices": [index if index >= 0 else None for index in raw_predecessors],
        "edges": [tuple(int(index) for index in edge) for edge in payload["edges"]],
        "positions": [float(position) for position in payload["positions"]],
        "points": [tuple(float(coordinate) for coordinate in point) for point in payload["points"]],
        "segment_lengths": [float(length) for length in payload["segment_lengths"]],
        "length_mm": float(payload["length_mm"]),
        "reached_vertex": None if payload["reached_vertex"] is None else int(payload["reached_vertex"]),
        "stopped_reason": str(payload["stopped_reason"]),
        "steps": int(payload["steps"]),
        "meshlib_reference": str(payload.get("meshlib_reference", "MR::computeFastMarchingPath")),
    }


def mesh_fast_marching_surface_path_tri_points(
    mesh: MeshDocument,
    *,
    start_face_index: int,
    start_barycentric: tuple[float, float, float],
    end_face_index: int,
    end_barycentric: tuple[float, float, float],
    max_steps: int = 1024,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_fast_marching_surface_path_tri_points")
    payload: dict[str, Any] = kernel(
        mesh.vertices,
        mesh.faces,
        int(start_face_index),
        tuple(float(value) for value in start_barycentric),
        int(end_face_index),
        tuple(float(value) for value in end_barycentric),
        int(max_steps),
    )
    raw_predecessors = [int(index) for index in payload["surface_predecessor_vertices"]]
    raw_distances = [float(distance) for distance in payload["surface_distances_mm"]]
    return {
        "start_face_index": int(payload["start_face_index"]),
        "start_barycentric": tuple(float(value) for value in payload["start_barycentric"]),
        "start_point": tuple(float(coordinate) for coordinate in payload["start_point"]),
        "end_face_index": int(payload["end_face_index"]),
        "end_barycentric": tuple(float(value) for value in payload["end_barycentric"]),
        "end_point": tuple(float(coordinate) for coordinate in payload["end_point"]),
        "surface_distances_mm": [distance if np.isfinite(distance) else None for distance in raw_distances],
        "surface_predecessor_vertices": [index if index >= 0 else None for index in raw_predecessors],
        "edges": [tuple(int(index) for index in edge) for edge in payload["edges"]],
        "positions": [float(position) for position in payload["positions"]],
        "points": [tuple(float(coordinate) for coordinate in point) for point in payload["points"]],
        "segment_lengths": [float(length) for length in payload["segment_lengths"]],
        "length_mm": float(payload["length_mm"]),
        "reached_face_index": None
        if payload["reached_face_index"] is None
        else int(payload["reached_face_index"]),
        "stopped_reason": str(payload["stopped_reason"]),
        "steps": int(payload["steps"]),
        "meshlib_reference": str(payload.get("meshlib_reference", "MR::computeFastMarchingPath")),
    }


def mesh_surface_path_tri_points(
    mesh: MeshDocument,
    *,
    start_face_index: int,
    start_barycentric: tuple[float, float, float],
    end_face_index: int,
    end_barycentric: tuple[float, float, float],
    max_geodesic_iters: int = 5,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_surface_path_tri_points")
    payload: dict[str, Any] = kernel(
        mesh.vertices,
        mesh.faces,
        int(start_face_index),
        tuple(float(value) for value in start_barycentric),
        int(end_face_index),
        tuple(float(value) for value in end_barycentric),
        int(max_geodesic_iters),
    )
    raw_predecessors = [int(index) for index in payload["surface_predecessor_vertices"]]
    raw_distances = [float(distance) for distance in payload["surface_distances_mm"]]
    return {
        "start_face_index": int(payload["start_face_index"]),
        "start_barycentric": tuple(float(value) for value in payload["start_barycentric"]),
        "start_point": tuple(float(coordinate) for coordinate in payload["start_point"]),
        "end_face_index": int(payload["end_face_index"]),
        "end_barycentric": tuple(float(value) for value in payload["end_barycentric"]),
        "end_point": tuple(float(coordinate) for coordinate in payload["end_point"]),
        "surface_distances_mm": [distance if np.isfinite(distance) else None for distance in raw_distances],
        "surface_predecessor_vertices": [index if index >= 0 else None for index in raw_predecessors],
        "approximate_edges": [tuple(int(index) for index in edge) for edge in payload["approximate_edges"]],
        "approximate_positions": [float(position) for position in payload["approximate_positions"]],
        "approximate_points": [tuple(float(coordinate) for coordinate in point) for point in payload["approximate_points"]],
        "edges": [tuple(int(index) for index in edge) for edge in payload["edges"]],
        "positions": [float(position) for position in payload["positions"]],
        "points": [tuple(float(coordinate) for coordinate in point) for point in payload["points"]],
        "segment_lengths": [float(length) for length in payload["segment_lengths"]],
        "length_mm": float(payload["length_mm"]),
        "reached_face_index": None
        if payload["reached_face_index"] is None
        else int(payload["reached_face_index"]),
        "stopped_reason": str(payload["stopped_reason"]),
        "reduce_iterations": int(payload["reduce_iterations"]),
        "steps": int(payload["steps"]),
        "meshlib_reference": str(payload.get("meshlib_reference", "MR::computeSurfacePath / MR::reducePath")),
    }
