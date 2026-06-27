from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.accelerators._rust_geodesic_surface import (
    mesh_closest_surface_path_targets,
    mesh_geodesic_distance_field,
    mesh_geodesic_iso_region,
    mesh_surface_distance_seed_vertices,
)
from geometry_sdk.types import MeshDocument


def _require_core_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def mesh_geodesic_path(
    mesh: MeshDocument,
    *,
    start_vertex: int,
    end_vertex: int,
    max_path_len_mm: float | None = None,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_geodesic_path")
    limit = np.finfo(np.float64).max if max_path_len_mm is None else float(max_path_len_mm)
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, int(start_vertex), int(end_vertex), limit)
    return {
        "vertex_indices": [int(index) for index in payload["vertex_indices"]],
        "points": [tuple(float(coordinate) for coordinate in point) for point in payload["points"]],
        "point_normals": [tuple(float(coordinate) for coordinate in normal) for normal in payload["point_normals"]],
        "edge_lengths": [float(length) for length in payload["edge_lengths"]],
        "length_mm": float(payload["length_mm"]),
        "line_segments": int(payload["line_segments"]),
        "meshlib_reference": str(payload.get("meshlib_reference", "MR::buildShortestPath")),
    }


def mesh_geodesic_polyline_path(
    mesh: MeshDocument,
    *,
    control_vertices,
    close_path: bool = False,
    max_path_len_mm: float | None = None,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_geodesic_polyline_path")
    controls = np.asarray(control_vertices, dtype=np.int64).reshape((-1,))
    limit = np.finfo(np.float64).max if max_path_len_mm is None else float(max_path_len_mm)
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, controls, bool(close_path), limit)
    return {
        "control_vertex_indices": [int(index) for index in payload["control_vertex_indices"]],
        "control_vertex_offsets": [int(index) for index in payload["control_vertex_offsets"]],
        "vertex_indices": [int(index) for index in payload["vertex_indices"]],
        "points": [tuple(float(coordinate) for coordinate in point) for point in payload["points"]],
        "point_normals": [tuple(float(coordinate) for coordinate in normal) for normal in payload["point_normals"]],
        "edge_lengths": [float(length) for length in payload["edge_lengths"]],
        "leg_lengths": [float(length) for length in payload["leg_lengths"]],
        "leg_vertex_offsets": [int(index) for index in payload["leg_vertex_offsets"]],
        "length_mm": float(payload["length_mm"]),
        "line_segments": int(payload["line_segments"]),
        "closed_path": bool(payload["closed_path"]),
        "meshlib_reference": str(payload.get("meshlib_reference", "MR::buildShortestPath control polyline")),
    }


def mesh_cut_measure_contours(
    mesh: MeshDocument,
    *,
    control_vertices,
    close_path: bool = False,
    max_path_len_mm: float | None = None,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_cut_measure_contours")
    controls = np.asarray(control_vertices, dtype=np.int64).reshape((-1,))
    limit = np.finfo(np.float64).max if max_path_len_mm is None else float(max_path_len_mm)
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, controls, bool(close_path), limit)
    return {
        "closed_path": bool(payload["closed_path"]),
        "contour_count": int(payload["contour_count"]),
        "cut_result_count": int(payload["cut_result_count"]),
        "path_vertex_indices": [int(index) for index in payload["path_vertex_indices"]],
        "path_points": [tuple(float(coordinate) for coordinate in point) for point in payload["path_points"]],
        "edge_lengths": [float(length) for length in payload["edge_lengths"]],
        "length_mm": float(payload["length_mm"]),
        "line_segments": int(payload["line_segments"]),
        "pivot_indices": [int(index) for index in payload["pivot_indices"]],
        "result_cut_vertex_indices": [
            [int(index) for index in path]
            for path in payload["result_cut_vertex_indices"]
        ],
        "bad_face_indices": [int(index) for index in payload["bad_face_indices"]],
        "contours": [
            {
                "closed": bool(contour["closed"]),
                "intersections": [
                    {
                        "primitive_type": str(intersection["primitive_type"]),
                        "primitive_id": int(intersection["primitive_id"]),
                        "coordinate": tuple(float(coordinate) for coordinate in intersection["coordinate"]),
                    }
                    for intersection in contour["intersections"]
                ],
            }
            for contour in payload["contours"]
        ],
        "meshlib_reference": str(
            payload.get("meshlib_reference", "MR::convertSurfacePathsToMeshContours / MR::cutMesh")
        ),
    }


def mesh_cut_measure_edge_path_topology_cut(
    mesh: MeshDocument,
    *,
    control_vertices,
    close_path: bool = False,
    max_path_len_mm: float | None = None,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_cut_measure_edge_path_topology_cut")
    controls = np.asarray(control_vertices, dtype=np.int64).reshape((-1,))
    limit = np.finfo(np.float64).max if max_path_len_mm is None else float(max_path_len_mm)
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, controls, bool(close_path), limit)
    metadata = dict(mesh.metadata)
    metadata.update(
        {
            "source": "rust_mesh_cut_measure_edge_path_topology_cut",
            "meshlib_reference": str(
                payload.get(
                    "meshlib_reference",
                    "MR::convertSurfacePathsToMeshContours / MR::cutMesh edge-path seam subset",
                )
            ),
            "meshlib_source": "MeshLib/source/MRMesh/MROneMeshContours.*; MeshLib/source/MRMesh/MRContoursCut.*",
            "rust_backed": True,
        }
    )
    output_mesh = MeshDocument(
        vertices=np.asarray(payload["vertices"], dtype=np.float64),
        faces=np.asarray(payload["faces"], dtype=np.int64),
        unit=mesh.unit,
        metadata=metadata,
    )
    return {
        "mesh": output_mesh,
        "source_path_vertex_indices": [int(index) for index in payload["source_path_vertex_indices"]],
        "result_cut_vertex_indices": [
            [int(index) for index in path]
            for path in payload["result_cut_vertex_indices"]
        ],
        "duplicate_vertex_map": [
            [int(entry[0]), int(entry[1])] for entry in payload["duplicate_vertex_map"]
        ],
        "cut_edge_pairs": [[int(entry[0]), int(entry[1])] for entry in payload["cut_edge_pairs"]],
        "result_cut_edge_pairs": [
            [int(entry[0]), int(entry[1])] for entry in payload["result_cut_edge_pairs"]
        ],
        "bad_face_indices": [int(index) for index in payload["bad_face_indices"]],
        "closed_path": bool(payload["closed_path"]),
        "length_mm": float(payload["length_mm"]),
        "meshlib_reference": str(
            payload.get(
                "meshlib_reference",
                "MR::convertSurfacePathsToMeshContours / MR::cutMesh edge-path seam subset",
            )
        ),
    }


def mesh_geodesic_quadrangle_path(
    mesh: MeshDocument,
    *,
    start_vertex: int,
    end_vertex: int,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_geodesic_quadrangle_path")
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, int(start_vertex), int(end_vertex))
    return {
        "start_vertex": int(payload["start_vertex"]),
        "end_vertex": int(payload["end_vertex"]),
        "start_face_index": int(payload["start_face_index"]),
        "end_face_index": int(payload["end_face_index"]),
        "shared_edge": tuple(int(index) for index in payload["shared_edge"]),
        "crossing_t": float(payload["crossing_t"]),
        "crossing_point": tuple(float(coordinate) for coordinate in payload["crossing_point"]),
        "points": [tuple(float(coordinate) for coordinate in point) for point in payload["points"]],
        "edge_lengths": [float(length) for length in payload["edge_lengths"]],
        "length_mm": float(payload["length_mm"]),
        "graph_vertex_indices": [int(index) for index in payload["graph_vertex_indices"]],
        "graph_length_mm": float(payload["graph_length_mm"]),
        "unfolded_quadrangle_convex": bool(payload["unfolded_quadrangle_convex"]),
        "meshlib_reference": str(
            payload.get("meshlib_reference", "MR::shortestPathInQuadrangle / MR::reducePath")
        ),
    }


def mesh_planar_triangle_strip_path(
    *,
    start,
    portals,
    end,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_planar_triangle_strip_path")
    start_array = np.asarray(start, dtype=np.float64).reshape((2,))
    portal_array = np.asarray(portals, dtype=np.float64)
    portal_array = portal_array.reshape((0, 4)) if portal_array.size == 0 else portal_array.reshape((-1, 4))
    end_array = np.asarray(end, dtype=np.float64).reshape((2,))
    payload: dict[str, Any] = kernel(start_array, portal_array, end_array)
    return {
        "crossing_positions": [float(position) for position in payload["crossing_positions"]],
        "crossing_points": [tuple(float(coordinate) for coordinate in point) for point in payload["crossing_points"]],
        "points": [tuple(float(coordinate) for coordinate in point) for point in payload["points"]],
        "segment_lengths": [float(length) for length in payload["segment_lengths"]],
        "length_mm": float(payload["length_mm"]),
        "meshlib_reference": str(
            payload.get("meshlib_reference", "MR::PathInPlanarTriangleStrip / MR::reducePath")
        ),
    }


def mesh_surface_edge_point_path(
    mesh: MeshDocument,
    *,
    edges,
    positions,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_surface_edge_point_path")
    edge_array = np.asarray(edges, dtype=np.int64)
    edge_array = edge_array.reshape((0, 2)) if edge_array.size == 0 else edge_array.reshape((-1, 2))
    position_array = np.asarray(positions, dtype=np.float64).reshape((-1,))
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, edge_array, position_array)
    return {
        "edges": [tuple(int(index) for index in edge) for edge in payload["edges"]],
        "positions": [float(position) for position in payload["positions"]],
        "points": [tuple(float(coordinate) for coordinate in point) for point in payload["points"]],
        "segment_lengths": [float(length) for length in payload["segment_lengths"]],
        "length_mm": float(payload["length_mm"]),
        "meshlib_reference": str(
            payload.get("meshlib_reference", "MR::surfacePathLength / MR::surfacePathToContour3f")
        ),
    }


def mesh_geodesic_edge_point_path(
    mesh: MeshDocument,
    *,
    start_point,
    edges,
    positions,
    end_point,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_geodesic_edge_point_path")
    start_array = np.asarray(start_point, dtype=np.float64).reshape((3,))
    edge_array = np.asarray(edges, dtype=np.int64)
    edge_array = edge_array.reshape((0, 2)) if edge_array.size == 0 else edge_array.reshape((-1, 2))
    position_array = np.asarray(positions, dtype=np.float64).reshape((-1,))
    end_array = np.asarray(end_point, dtype=np.float64).reshape((3,))
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, start_array, edge_array, position_array, end_array)
    return {
        "start_point": tuple(float(coordinate) for coordinate in payload["start_point"]),
        "end_point": tuple(float(coordinate) for coordinate in payload["end_point"]),
        "edges": [tuple(int(index) for index in edge) for edge in payload["edges"]],
        "positions": [float(position) for position in payload["positions"]],
        "mid_points": [tuple(float(coordinate) for coordinate in point) for point in payload["mid_points"]],
        "points": [tuple(float(coordinate) for coordinate in point) for point in payload["points"]],
        "segment_lengths": [float(length) for length in payload["segment_lengths"]],
        "length_mm": float(payload["length_mm"]),
        "meshlib_reference": str(
            payload.get("meshlib_reference", "MR::geodesicPathLength / MR::geodesicPathToContour3f")
        ),
    }


def mesh_triangle_strip_unfolded_path(
    mesh: MeshDocument,
    *,
    start_face_index: int,
    crossed_edges,
    end_face_index: int,
    start_point,
    end_point,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_triangle_strip_unfolded_path")
    edge_array = np.asarray(crossed_edges, dtype=np.int64)
    edge_array = edge_array.reshape((0, 2)) if edge_array.size == 0 else edge_array.reshape((-1, 2))
    start_array = np.asarray(start_point, dtype=np.float64).reshape((3,))
    end_array = np.asarray(end_point, dtype=np.float64).reshape((3,))
    payload: dict[str, Any] = kernel(
        mesh.vertices,
        mesh.faces,
        int(start_face_index),
        edge_array,
        int(end_face_index),
        start_array,
        end_array,
    )
    return {
        "start_face_index": int(payload["start_face_index"]),
        "end_face_index": int(payload["end_face_index"]),
        "strip_face_indices": [int(index) for index in payload["strip_face_indices"]],
        "crossed_edges": [tuple(int(index) for index in edge) for edge in payload["crossed_edges"]],
        "oriented_edges": [tuple(int(index) for index in edge) for edge in payload["oriented_edges"]],
        "crossing_positions": [float(position) for position in payload["crossing_positions"]],
        "crossing_points": [tuple(float(coordinate) for coordinate in point) for point in payload["crossing_points"]],
        "points": [tuple(float(coordinate) for coordinate in point) for point in payload["points"]],
        "segment_lengths": [float(length) for length in payload["segment_lengths"]],
        "length_mm": float(payload["length_mm"]),
        "planar_length_mm": float(payload["planar_length_mm"]),
        "meshlib_reference": str(payload.get("meshlib_reference", "MR::TriangleStripUnfolder / MR::reducePath")),
    }


def mesh_steepest_descent_triangle_step(
    mesh: MeshDocument,
    *,
    vertex_scalars,
    face_index: int,
    start_barycentric,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_steepest_descent_triangle_step")
    scalars = np.asarray(vertex_scalars, dtype=np.float64).reshape((-1,))
    barycentric = np.asarray(start_barycentric, dtype=np.float64).reshape((3,))
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, scalars, int(face_index), barycentric)
    crossed_edge = payload["crossed_edge"]
    crossing_point = payload["crossing_point"]
    return {
        "face_index": int(payload["face_index"]),
        "start_barycentric": tuple(float(value) for value in payload["start_barycentric"]),
        "start_point": tuple(float(coordinate) for coordinate in payload["start_point"]),
        "start_value": float(payload["start_value"]),
        "gradient": tuple(float(coordinate) for coordinate in payload["gradient"]),
        "gradient_norm": float(payload["gradient_norm"]),
        "crossed_edge": None if crossed_edge is None else tuple(int(index) for index in crossed_edge),
        "edge_position": None if payload["edge_position"] is None else float(payload["edge_position"]),
        "crossing_point": None if crossing_point is None else tuple(float(coordinate) for coordinate in crossing_point),
        "kind": str(payload["kind"]),
        "meshlib_reference": str(
            payload.get("meshlib_reference", "MR::findSteepestDescentPoint(MeshTriPoint)")
        ),
    }


def mesh_steepest_descent_edge_step(
    mesh: MeshDocument,
    *,
    vertex_scalars,
    edge,
    edge_position: float,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_steepest_descent_edge_step")
    scalars = np.asarray(vertex_scalars, dtype=np.float64).reshape((-1,))
    edge_array = np.asarray(edge, dtype=np.int64).reshape((2,))
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, scalars, edge_array, float(edge_position))
    crossed_edge = payload["crossed_edge"]
    crossing_point = payload["crossing_point"]
    return {
        "start_edge": tuple(int(index) for index in payload["start_edge"]),
        "edge_position": float(payload["edge_position"]),
        "start_point": tuple(float(coordinate) for coordinate in payload["start_point"]),
        "start_value": float(payload["start_value"]),
        "crossed_edge": None if crossed_edge is None else tuple(int(index) for index in crossed_edge),
        "crossing_edge_position": None
        if payload["crossing_edge_position"] is None
        else float(payload["crossing_edge_position"]),
        "crossing_point": None if crossing_point is None else tuple(float(coordinate) for coordinate in crossing_point),
        "kind": str(payload["kind"]),
        "side": str(payload["side"]),
        "meshlib_reference": str(
            payload.get("meshlib_reference", "MR::findSteepestDescentPoint(MeshEdgePoint)")
        ),
    }


def mesh_steepest_descent_vertex_step(
    mesh: MeshDocument,
    *,
    vertex_scalars,
    vertex_index: int,
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_steepest_descent_vertex_step")
    scalars = np.asarray(vertex_scalars, dtype=np.float64).reshape((-1,))
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, scalars, int(vertex_index))
    crossed_edge = payload["crossed_edge"]
    crossing_point = payload["crossing_point"]
    return {
        "start_vertex": int(payload["start_vertex"]),
        "start_point": tuple(float(coordinate) for coordinate in payload["start_point"]),
        "start_value": float(payload["start_value"]),
        "crossed_edge": None if crossed_edge is None else tuple(int(index) for index in crossed_edge),
        "edge_position": None if payload["edge_position"] is None else float(payload["edge_position"]),
        "crossing_point": None if crossing_point is None else tuple(float(coordinate) for coordinate in crossing_point),
        "gradient_norm": None if payload["gradient_norm"] is None else float(payload["gradient_norm"]),
        "kind": str(payload["kind"]),
        "source": str(payload["source"]),
        "meshlib_reference": str(payload.get("meshlib_reference", "MR::findSteepestDescentPoint(VertId)")),
    }


def mesh_steepest_descent_path(mesh: MeshDocument, *, vertex_scalars, face_index: int, start_barycentric, max_steps: int = 1024) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_steepest_descent_path")
    scalars = np.asarray(vertex_scalars, dtype=np.float64).reshape((-1,))
    start = np.asarray(start_barycentric, dtype=np.float64).reshape((3,))
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, scalars, int(face_index), start, int(max_steps))
    return {
        "start_face_index": int(payload["start_face_index"]),
        "start_barycentric": tuple(float(value) for value in payload["start_barycentric"]),
        "start_point": tuple(float(coordinate) for coordinate in payload["start_point"]),
        "start_value": float(payload["start_value"]),
        "edges": [tuple(int(index) for index in edge) for edge in payload["edges"]],
        "positions": [float(position) for position in payload["positions"]],
        "points": [tuple(float(coordinate) for coordinate in point) for point in payload["points"]],
        "segment_lengths": [float(length) for length in payload["segment_lengths"]],
        "length_mm": float(payload["length_mm"]),
        "reached_vertex": None if payload["reached_vertex"] is None else int(payload["reached_vertex"]),
        "stopped_reason": str(payload["stopped_reason"]),
        "steps": int(payload["steps"]),
        "meshlib_reference": str(payload.get("meshlib_reference", "MR::computeSteepestDescentPath")),
    }


def mesh_geodesic_extreme_edges(
    mesh: MeshDocument,
    *,
    scalars,
    extreme_type: str = "ridge",
) -> dict[str, Any]:
    kernel = _require_core_kernel("mesh_geodesic_extreme_edges")
    field = np.asarray(scalars, dtype=np.float64).reshape((-1,))
    payload: dict[str, Any] = kernel(mesh.vertices, mesh.faces, field, str(extreme_type))
    return {
        "extreme_type": str(payload["extreme_type"]),
        "edge_indices": [tuple(int(index) for index in edge) for edge in payload["edge_indices"]],
        "meshlib_reference": str(payload.get("meshlib_reference", "MR::findExtremeEdges")),
    }
