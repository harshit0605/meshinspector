from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def point_cloud_grid_sample_indices(
    points: np.ndarray,
    *,
    voxel_size: float,
    max_voxels: int = 500_000,
) -> np.ndarray | None:
    kernel = _require_rust_kernel("point_cloud_grid_sample_indices")
    return kernel(
        np.asarray(points, dtype=np.float64),
        float(voxel_size),
        int(max_voxels),
    )


def point_cloud_uniform_sample_indices(
    points: np.ndarray,
    *,
    distance: float,
    min_normal_dot: float = 0.0,
    lexicographical_order: bool = True,
    normals: np.ndarray | None = None,
) -> np.ndarray | None:
    kernel = _require_rust_kernel("point_cloud_uniform_sample_indices")
    return kernel(
        np.asarray(points, dtype=np.float64),
        float(distance),
        float(min_normal_dot),
        bool(lexicographical_order),
        None if normals is None else np.asarray(normals, dtype=np.float64),
    )


def point_cloud_nearest_projections(
    query_points: np.ndarray,
    reference_points: np.ndarray,
    *,
    up_dist_limit_sq: float = np.inf,
    lo_dist_limit_sq: float = 0.0,
    skip_same_index: bool = False,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("point_cloud_nearest_projections")
    return kernel(
        np.asarray(query_points, dtype=np.float64),
        np.asarray(reference_points, dtype=np.float64),
        float(up_dist_limit_sq),
        float(lo_dist_limit_sq),
        bool(skip_same_index),
    )


def point_cloud_project_to_mesh(
    query_points: np.ndarray,
    mesh_vertices: np.ndarray,
    mesh_faces: np.ndarray,
    *,
    up_dist_limit_sq: float = np.finfo(np.float64).max,
    lo_dist_limit_sq: float = 0.0,
    point_transform: np.ndarray | None = None,
    mesh_transform: np.ndarray | None = None,
    face_mask: np.ndarray | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("point_cloud_project_to_mesh")
    return kernel(
        np.asarray(query_points, dtype=np.float64),
        np.asarray(mesh_vertices, dtype=np.float64),
        np.asarray(mesh_faces, dtype=np.int64),
        float(up_dist_limit_sq),
        float(lo_dist_limit_sq),
        None
        if point_transform is None
        else np.asarray(point_transform, dtype=np.float64).reshape(-1).tolist(),
        None
        if mesh_transform is None
        else np.asarray(mesh_transform, dtype=np.float64).reshape(-1).tolist(),
        None
        if face_mask is None
        else np.asarray(face_mask, dtype=np.bool_).reshape(-1).tolist(),
    )


def point_cloud_n_closest_neighbors(
    points: np.ndarray,
    *,
    num_neighbors: int,
    up_dist_limit_sq: float = np.finfo(np.float64).max,
) -> np.ndarray | None:
    kernel = _require_rust_kernel("point_cloud_n_closest_neighbors")
    return kernel(
        np.asarray(points, dtype=np.float64),
        int(num_neighbors),
        float(up_dist_limit_sq),
    )


def point_cloud_two_closest_points(points: np.ndarray) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("point_cloud_two_closest_points")
    return kernel(np.asarray(points, dtype=np.float64))


def point_cloud_neighbors_in_radius(
    points: np.ndarray,
    *,
    center_index: int,
    radius: float,
    normals: np.ndarray | None = None,
    untrusted_indices: np.ndarray | None = None,
) -> np.ndarray | None:
    kernel = _require_rust_kernel("point_cloud_neighbors_in_radius")
    return kernel(
        np.asarray(points, dtype=np.float64),
        int(center_index),
        float(radius),
        None if normals is None else np.asarray(normals, dtype=np.float64),
        None if untrusted_indices is None else np.asarray(untrusted_indices, dtype=np.int64),
    )


def point_cloud_select_by_screen_polygon(
    points: np.ndarray,
    view_projection_4x4: np.ndarray,
    polygon_xy: np.ndarray,
    *,
    normals: np.ndarray | None = None,
    include_backfaces: bool = True,
    visible_only: bool = False,
) -> np.ndarray | None:
    kernel = _require_rust_kernel("point_cloud_select_by_screen_polygon")
    return kernel(
        np.asarray(points, dtype=np.float64),
        np.asarray(view_projection_4x4, dtype=np.float64).reshape(-1),
        np.asarray(polygon_xy, dtype=np.float64),
        None if normals is None else np.asarray(normals, dtype=np.float64),
        bool(include_backfaces),
        bool(visible_only),
    )


def point_cloud_select_by_screen_rect(
    points: np.ndarray,
    view_projection_4x4: np.ndarray,
    rect_min_xy: np.ndarray,
    rect_max_xy: np.ndarray,
    *,
    normals: np.ndarray | None = None,
    include_backfaces: bool = True,
    visible_only: bool = False,
) -> np.ndarray | None:
    kernel = _require_rust_kernel("point_cloud_select_by_screen_rect")
    return kernel(
        np.asarray(points, dtype=np.float64),
        np.asarray(view_projection_4x4, dtype=np.float64).reshape(-1),
        np.asarray(rect_min_xy, dtype=np.float64).reshape(-1),
        np.asarray(rect_max_xy, dtype=np.float64).reshape(-1),
        None if normals is None else np.asarray(normals, dtype=np.float64),
        bool(include_backfaces),
        bool(visible_only),
    )


def point_cloud_select_by_screen_brush(
    points: np.ndarray,
    view_projection_4x4: np.ndarray,
    brush_path_xy: np.ndarray,
    *,
    radius_px: float,
    normals: np.ndarray | None = None,
    include_backfaces: bool = True,
    visible_only: bool = False,
) -> np.ndarray | None:
    kernel = _require_rust_kernel("point_cloud_select_by_screen_brush")
    return kernel(
        np.asarray(points, dtype=np.float64),
        np.asarray(view_projection_4x4, dtype=np.float64).reshape(-1),
        np.asarray(brush_path_xy, dtype=np.float64),
        float(radius_px),
        None if normals is None else np.asarray(normals, dtype=np.float64),
        bool(include_backfaces),
        bool(visible_only),
    )


def point_cloud_pick_by_ray(
    points: np.ndarray,
    ray_origin: np.ndarray,
    ray_direction: np.ndarray,
    *,
    max_distance_to_ray: float,
    max_depth: float = np.inf,
    normals: np.ndarray | None = None,
    include_backfaces: bool = True,
) -> np.ndarray | None:
    kernel = _require_rust_kernel("point_cloud_pick_by_ray")
    return kernel(
        np.asarray(points, dtype=np.float64),
        np.asarray(ray_origin, dtype=np.float64).reshape(-1),
        np.asarray(ray_direction, dtype=np.float64).reshape(-1),
        float(max_distance_to_ray),
        float(max_depth),
        None if normals is None else np.asarray(normals, dtype=np.float64),
        bool(include_backfaces),
    )


def point_cloud_extract_selected_points_as_object(
    points: np.ndarray,
    selected_point_ids,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("point_cloud_extract_selected_points_as_object")
    return kernel(
        np.asarray(points, dtype=np.float64),
        np.asarray(selected_point_ids, dtype=np.int64).reshape(-1),
    )


def point_cloud_local_neighbor_fan(
    points: np.ndarray,
    *,
    center_index: int,
    radius: float,
    num_neighbors: int = 0,
    boundary_angle: float = np.pi * 0.9,
    max_removes: int = 0,
    crit_angle: float = np.pi * 2.0,
    normals: np.ndarray | None = None,
    untrusted_indices: np.ndarray | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("point_cloud_local_neighbor_fan")
    return kernel(
        np.asarray(points, dtype=np.float64),
        int(center_index),
        float(radius),
        int(num_neighbors),
        float(boundary_angle),
        int(max_removes),
        float(crit_angle),
        None if normals is None else np.asarray(normals, dtype=np.float64),
        None if untrusted_indices is None else np.asarray(untrusted_indices, dtype=np.int64),
    )


def point_cloud_local_fan_triangles(
    points: np.ndarray,
    *,
    center_index: int,
    radius: float,
    num_neighbors: int = 0,
    boundary_angle: float = np.pi * 0.9,
    max_removes: int = 0,
    crit_angle: float = np.pi * 2.0,
    normals: np.ndarray | None = None,
    untrusted_indices: np.ndarray | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("point_cloud_local_fan_triangles")
    return kernel(
        np.asarray(points, dtype=np.float64),
        int(center_index),
        float(radius),
        int(num_neighbors),
        float(boundary_angle),
        int(max_removes),
        float(crit_angle),
        None if normals is None else np.asarray(normals, dtype=np.float64),
        None if untrusted_indices is None else np.asarray(untrusted_indices, dtype=np.int64),
    )


def point_cloud_local_triangulation_repetitions(
    points: np.ndarray,
    *,
    radius: float,
    num_neighbors: int = 0,
    boundary_angle: float = np.pi * 0.9,
    max_removes: int = 0,
    crit_angle: float = np.pi * 2.0,
    normals: np.ndarray | None = None,
    untrusted_indices: np.ndarray | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("point_cloud_local_triangulation_repetitions")
    return kernel(
        np.asarray(points, dtype=np.float64),
        float(radius),
        int(num_neighbors),
        float(boundary_angle),
        int(max_removes),
        float(crit_angle),
        None if normals is None else np.asarray(normals, dtype=np.float64),
        None if untrusted_indices is None else np.asarray(untrusted_indices, dtype=np.int64),
    )


def point_cloud_triangulate_candidate_mesh(
    points: np.ndarray,
    *,
    radius: float = 0.0,
    num_neighbors: int = 16,
    boundary_angle: float = np.pi * 0.9,
    max_removes: int = 2_147_483_647,
    crit_angle: float = np.pi * 2.0,
    normals: np.ndarray | None = None,
    untrusted_indices: np.ndarray | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("point_cloud_triangulate_candidate_mesh")
    return kernel(
        np.asarray(points, dtype=np.float64),
        float(radius),
        int(num_neighbors),
        float(boundary_angle),
        int(max_removes),
        float(crit_angle),
        None if normals is None else np.asarray(normals, dtype=np.float64),
        None if untrusted_indices is None else np.asarray(untrusted_indices, dtype=np.int64),
    )


def point_cloud_triangulate_cleaned_candidate_mesh(
    points: np.ndarray,
    *,
    radius: float = 0.0,
    num_neighbors: int = 16,
    boundary_angle: float = np.pi * 0.9,
    max_removes: int = 2_147_483_647,
    crit_angle: float = np.pi * 2.0,
    normals: np.ndarray | None = None,
    untrusted_indices: np.ndarray | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("point_cloud_triangulate_cleaned_candidate_mesh")
    return kernel(
        np.asarray(points, dtype=np.float64),
        float(radius),
        int(num_neighbors),
        float(boundary_angle),
        int(max_removes),
        float(crit_angle),
        None if normals is None else np.asarray(normals, dtype=np.float64),
        None if untrusted_indices is None else np.asarray(untrusted_indices, dtype=np.int64),
    )


def point_cloud_triangulate_topology_candidate_mesh(
    points: np.ndarray,
    *,
    radius: float = 0.0,
    num_neighbors: int = 16,
    boundary_angle: float = np.pi * 0.9,
    max_removes: int = 2_147_483_647,
    crit_angle: float = np.pi * 2.0,
    normals: np.ndarray | None = None,
    untrusted_indices: np.ndarray | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("point_cloud_triangulate_topology_candidate_mesh")
    return kernel(
        np.asarray(points, dtype=np.float64),
        float(radius),
        int(num_neighbors),
        float(boundary_angle),
        int(max_removes),
        float(crit_angle),
        None if normals is None else np.asarray(normals, dtype=np.float64),
        None if untrusted_indices is None else np.asarray(untrusted_indices, dtype=np.int64),
    )


def point_cloud_triangulate_filled_candidate_mesh(
    points: np.ndarray,
    *,
    radius: float = 0.0,
    num_neighbors: int = 16,
    boundary_angle: float = np.pi * 0.9,
    max_removes: int = 2_147_483_647,
    crit_angle: float = np.pi * 2.0,
    crit_hole_length: float = -1.0,
    normals: np.ndarray | None = None,
    untrusted_indices: np.ndarray | None = None,
) -> dict[str, Any] | None:
    kernel = _require_rust_kernel("point_cloud_triangulate_filled_candidate_mesh")
    return kernel(
        np.asarray(points, dtype=np.float64),
        float(radius),
        int(num_neighbors),
        float(boundary_angle),
        int(max_removes),
        float(crit_angle),
        float(crit_hole_length),
        None if normals is None else np.asarray(normals, dtype=np.float64),
        None if untrusted_indices is None else np.asarray(untrusted_indices, dtype=np.int64),
    )
