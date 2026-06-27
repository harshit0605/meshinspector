from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.accelerators._rust_mesh_io import mesh_from_obj, mesh_from_ply, mesh_to_ply_bytes
from geometry_sdk.accelerators._rust_mesh_selection import extract_selected_faces_as_mesh
from geometry_sdk.accelerators._rust_mesh_scene import (
    meshlib_multi_object_mru_scene_bytes,
    meshlib_object_mesh_mru_scene_bytes,
    meshlib_object_mesh_scene_json,
    meshlib_object_mesh_scene_payload,
    mesh_from_mru_scene,
)
from geometry_sdk.accelerators._rust_mesh_scene_edit import (
    meshlib_reorder_scene_children,
    meshlib_reparent_scene_object,
    meshlib_scene_feature_object_render_payload,
    meshlib_select_scene_objects,
    meshlib_set_scene_feature_object_visualize_property,
    meshlib_set_scene_object_state,
    meshlib_transform_scene_object,
)
from geometry_sdk.accelerators._rust_mesh_scene_ribbon import (
    meshlib_apply_scene_ribbon_action,
    meshlib_group_scene_objects,
    meshlib_rename_scene_object,
    meshlib_ungroup_scene_objects,
)
from geometry_sdk.types import MeshDocument, MeshHealth, MeshStats


def _require_rust_kernel(name: str):
    _common.accelerator_mode()
    if _common._rs is None:
        raise RuntimeError("Rust geometry backend requires _zennah_geometry_rs, but it is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust geometry backend requires _zennah_geometry_rs.{name}, but it is not exposed")
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


def select_boundary_faces(mesh: MeshDocument) -> list[int]:
    kernel = _require_core_kernel("select_boundary_faces")
    return [int(face_id) for face_id in kernel(mesh.vertices, mesh.faces)]


def select_boundary_edges(mesh: MeshDocument) -> list[tuple[int, int]]:
    kernel = _require_core_kernel("select_boundary_edges")
    edges = np.asarray(kernel(mesh.vertices, mesh.faces), dtype=np.int64).reshape((-1, 2))
    return [(int(edge[0]), int(edge[1])) for edge in edges]


def select_camera_facing_faces(
    mesh: MeshDocument,
    *,
    camera_direction,
    min_dot: float = 0.0,
) -> list[int]:
    kernel = _require_core_kernel("select_camera_facing_faces")
    camera_direction_values = np.asarray(camera_direction, dtype=np.float64).reshape((3,))
    return [
        int(face_id)
        for face_id in kernel(
            mesh.vertices,
            mesh.faces,
            camera_direction_values,
            float(min_dot),
        )
    ]


def select_overhang_faces(
    mesh: MeshDocument,
    *,
    axis=(0.0, 0.0, 1.0),
    layer_height_mm: float,
    max_overhang_distance_mm: float,
    hops: int = 0,
) -> list[int]:
    kernel = _require_core_kernel("select_overhang_faces")
    axis_values = np.asarray(axis, dtype=np.float64).reshape((3,))
    return [
        int(face_id)
        for face_id in kernel(
            mesh.vertices,
            mesh.faces,
            axis_values,
            float(layer_height_mm),
            float(max_overhang_distance_mm),
            int(hops),
        )
    ]


def select_outer_layer_faces(mesh: MeshDocument, *, epsilon: float = 1e-8) -> list[int]:
    kernel = _require_core_kernel("select_outer_layer_faces")
    return [int(face_id) for face_id in kernel(mesh.vertices, mesh.faces, float(epsilon))]


def select_overlapping_faces(
    mesh: MeshDocument,
    *,
    max_dist_sq: float = 1e-10,
    max_normal_dot: float = -0.99,
    min_area_fraction: float = 1e-5,
) -> list[int]:
    kernel = _require_core_kernel("select_overlapping_faces")
    return [
        int(face_id)
        for face_id in kernel(
            mesh.vertices,
            mesh.faces,
            float(max_dist_sq),
            float(max_normal_dot),
            float(min_area_fraction),
        )
    ]


def graph_cut_select_region(
    mesh: MeshDocument,
    *,
    source_face_ids,
    sink_face_ids,
    boundary_weight: float = 1.0,
    curvature_preference: str = "geodesic",
) -> list[int]:
    kernel = _require_core_kernel("graph_cut_select_region")
    source = np.asarray(source_face_ids, dtype=np.int64).reshape((-1,))
    sink = np.asarray(sink_face_ids, dtype=np.int64).reshape((-1,))
    return [
        int(face_id)
        for face_id in kernel(
            mesh.vertices,
            mesh.faces,
            source,
            sink,
            float(boundary_weight),
            str(curvature_preference),
        )
    ]


def graph_cut_select_region_auto_not_region(
    mesh: MeshDocument,
    *,
    source_face_ids,
    uncertainty_distance_mm: float,
    boundary_weight: float = 1.0,
    curvature_preference: str = "geodesic",
) -> list[int]:
    kernel = _require_core_kernel("graph_cut_select_region_auto_not_region")
    source = np.asarray(source_face_ids, dtype=np.int64).reshape((-1,))
    return [
        int(face_id)
        for face_id in kernel(
            mesh.vertices,
            mesh.faces,
            source,
            float(uncertainty_distance_mm),
            float(boundary_weight),
            str(curvature_preference),
        )
    ]


def select_faces_by_area(
    mesh: MeshDocument,
    *,
    area: float,
    scalar_type: str = "absolute",
    compare_type: str = "less",
) -> list[int]:
    kernel = _require_core_kernel("select_faces_by_area")
    return [
        int(face_id)
        for face_id in kernel(
            mesh.vertices,
            mesh.faces,
            float(area),
            str(scalar_type),
            str(compare_type),
        )
    ]


def face_adjacency(mesh: MeshDocument) -> list[list[int]]:
    kernel = _require_core_kernel("face_adjacency")
    return [[int(face_id) for face_id in neighbors] for neighbors in kernel(mesh.vertices, mesh.faces)]


def connected_face_components(mesh: MeshDocument) -> list[list[int]]:
    kernel = _require_core_kernel("connected_face_components")
    return [[int(face_id) for face_id in component] for component in kernel(mesh.vertices, mesh.faces)]


def select_largest_component_faces(mesh: MeshDocument, *, min_area_mm2: float = 0.0) -> list[int]:
    kernel = _require_core_kernel("select_largest_component_faces")
    return [int(face_id) for face_id in kernel(mesh.vertices, mesh.faces, float(min_area_mm2))]


def expand_face_selection_to_components(mesh: MeshDocument, seed_face_ids: list[int]) -> list[int]:
    kernel = _require_core_kernel("expand_face_selection_to_components")
    seeds = np.asarray(seed_face_ids, dtype=np.int64)
    return [int(face_id) for face_id in kernel(mesh.vertices, mesh.faces, seeds)]


def apply_meshlib_selection_modifier(
    current_ids,
    incoming_ids,
    mode: str,
    *,
    item_count: int | None = None,
) -> list[int]:
    kernel = _require_core_kernel("apply_meshlib_selection_modifier")
    current = np.asarray(current_ids, dtype=np.int64).reshape((-1,))
    incoming = np.asarray(incoming_ids, dtype=np.int64).reshape((-1,))
    return [int(face_id) for face_id in kernel(current, incoming, str(mode), item_count)]


def select_faces_by_screen_polygon(
    mesh: MeshDocument,
    view_projection_4x4,
    polygon_xy,
    *,
    include_backfaces: bool = True,
    visible_only: bool = False,
) -> list[int]:
    kernel = _require_core_kernel("select_faces_by_screen_polygon")
    view_projection = np.asarray(view_projection_4x4, dtype=np.float64).reshape((16,))
    polygon = np.asarray(polygon_xy, dtype=np.float64).reshape((-1, 2))
    return [
        int(face_id)
        for face_id in kernel(
            mesh.vertices,
            mesh.faces,
            view_projection,
            polygon,
            bool(include_backfaces),
            bool(visible_only),
        )
    ]


def select_faces_by_screen_rect(
    mesh: MeshDocument,
    view_projection_4x4,
    rect_min_xy,
    rect_max_xy,
    *,
    include_backfaces: bool = True,
    visible_only: bool = False,
) -> list[int]:
    kernel = _require_core_kernel("select_faces_by_screen_rect")
    view_projection = np.asarray(view_projection_4x4, dtype=np.float64).reshape((16,))
    rect_min = np.asarray(rect_min_xy, dtype=np.float64).reshape((2,))
    rect_max = np.asarray(rect_max_xy, dtype=np.float64).reshape((2,))
    return [
        int(face_id)
        for face_id in kernel(
            mesh.vertices,
            mesh.faces,
            view_projection,
            rect_min,
            rect_max,
            bool(include_backfaces),
            bool(visible_only),
        )
    ]


def select_faces_by_screen_brush(
    mesh: MeshDocument,
    view_projection_4x4,
    brush_path_xy,
    *,
    radius_px: float,
    include_backfaces: bool = True,
    visible_only: bool = False,
) -> list[int]:
    kernel = _require_core_kernel("select_faces_by_screen_brush")
    view_projection = np.asarray(view_projection_4x4, dtype=np.float64).reshape((16,))
    brush_path = np.asarray(brush_path_xy, dtype=np.float64).reshape((-1, 2))
    return [
        int(face_id)
        for face_id in kernel(
            mesh.vertices,
            mesh.faces,
            view_projection,
            brush_path,
            float(radius_px),
            bool(include_backfaces),
            bool(visible_only),
        )
    ]


def select_inside_part_faces(mesh: MeshDocument) -> list[int]:
    kernel = _require_core_kernel("select_inside_part_faces")
    return [int(face_id) for face_id in kernel(mesh.vertices, mesh.faces)]


def vertex_neighbors(mesh: MeshDocument) -> list[list[int]]:
    kernel = _require_core_kernel("vertex_neighbors")
    return [[int(vertex_id) for vertex_id in neighbors] for neighbors in kernel(mesh.vertices, mesh.faces)]


def mesh_stats(mesh: MeshDocument) -> MeshStats:
    kernel = _require_rust_kernel("mesh_stats")

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
