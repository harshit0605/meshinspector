"""Core mesh calculation compatibility wrappers."""

from __future__ import annotations

from pathlib import Path

from geometry_sdk.accelerators import (
    _rust_fast_marching,
    _rust_features,
    _rust_geodesic,
    _rust_mesh,
    _rust_mesh_selection,
    _rust_repair,
    _rust_smoothness,
    _rust_spatial,
    _rust_stats,
)
from geometry_sdk.types import MeshDocument, MeshStats


def safe_normalize(vectors):
    return _rust_mesh.safe_normalize(vectors)


def normalize_axis(axis):
    return _rust_mesh.normalize_axis(axis)


def bounds(mesh: MeshDocument):
    return _rust_mesh.bounds(mesh)


def face_normals(mesh: MeshDocument):
    return _rust_mesh.face_normals(mesh)


def vertex_normals(mesh: MeshDocument):
    return _rust_mesh.vertex_normals(mesh)


def surface_area(mesh: MeshDocument) -> float:
    return _rust_mesh.surface_area(mesh)


def signed_volume(mesh: MeshDocument) -> float:
    return _rust_mesh.signed_volume(mesh)


def volume(mesh: MeshDocument) -> float:
    return _rust_mesh.volume(mesh)


def mesh_geodesic_path(
    mesh: MeshDocument,
    *,
    start_vertex: int,
    end_vertex: int,
    max_path_len_mm: float | None = None,
) -> dict[str, object]:
    return _rust_geodesic.mesh_geodesic_path(
        mesh,
        start_vertex=start_vertex,
        end_vertex=end_vertex,
        max_path_len_mm=max_path_len_mm,
    )


def mesh_fast_marching_surface_path(
    mesh: MeshDocument,
    *,
    start_vertex: int,
    end_vertex: int,
    max_steps: int = 1024,
) -> dict[str, object]:
    return _rust_fast_marching.mesh_fast_marching_surface_path(
        mesh,
        start_vertex=start_vertex,
        end_vertex=end_vertex,
        max_steps=max_steps,
    )


def mesh_fast_marching_surface_path_tri_points(
    mesh: MeshDocument,
    *,
    start_face_index: int,
    start_barycentric: tuple[float, float, float],
    end_face_index: int,
    end_barycentric: tuple[float, float, float],
    max_steps: int = 1024,
) -> dict[str, object]:
    return _rust_fast_marching.mesh_fast_marching_surface_path_tri_points(
        mesh,
        start_face_index=start_face_index,
        start_barycentric=start_barycentric,
        end_face_index=end_face_index,
        end_barycentric=end_barycentric,
        max_steps=max_steps,
    )


def mesh_surface_path_tri_points(
    mesh: MeshDocument,
    *,
    start_face_index: int,
    start_barycentric: tuple[float, float, float],
    end_face_index: int,
    end_barycentric: tuple[float, float, float],
    max_geodesic_iters: int = 5,
) -> dict[str, object]:
    return _rust_fast_marching.mesh_surface_path_tri_points(
        mesh,
        start_face_index=start_face_index,
        start_barycentric=start_barycentric,
        end_face_index=end_face_index,
        end_barycentric=end_barycentric,
        max_geodesic_iters=max_geodesic_iters,
    )


def mesh_geodesic_polyline_path(
    mesh: MeshDocument,
    *,
    control_vertices,
    close_path: bool = False,
    max_path_len_mm: float | None = None,
) -> dict[str, object]:
    return _rust_geodesic.mesh_geodesic_polyline_path(
        mesh,
        control_vertices=control_vertices,
        close_path=close_path,
        max_path_len_mm=max_path_len_mm,
    )


def mesh_cut_measure_contours(
    mesh: MeshDocument,
    *,
    control_vertices,
    close_path: bool = False,
    max_path_len_mm: float | None = None,
) -> dict[str, object]:
    return _rust_geodesic.mesh_cut_measure_contours(
        mesh,
        control_vertices=control_vertices,
        close_path=close_path,
        max_path_len_mm=max_path_len_mm,
    )


def mesh_cut_measure_edge_path_topology_cut(
    mesh: MeshDocument,
    *,
    control_vertices,
    close_path: bool = False,
    max_path_len_mm: float | None = None,
) -> dict[str, object]:
    return _rust_geodesic.mesh_cut_measure_edge_path_topology_cut(
        mesh,
        control_vertices=control_vertices,
        close_path=close_path,
        max_path_len_mm=max_path_len_mm,
    )


def mesh_geodesic_quadrangle_path(
    mesh: MeshDocument,
    *,
    start_vertex: int,
    end_vertex: int,
) -> dict[str, object]:
    return _rust_geodesic.mesh_geodesic_quadrangle_path(
        mesh,
        start_vertex=start_vertex,
        end_vertex=end_vertex,
    )


def mesh_planar_triangle_strip_path(
    *,
    start,
    portals,
    end,
) -> dict[str, object]:
    return _rust_geodesic.mesh_planar_triangle_strip_path(
        start=start,
        portals=portals,
        end=end,
    )


def mesh_surface_edge_point_path(
    mesh: MeshDocument,
    *,
    edges,
    positions,
) -> dict[str, object]:
    return _rust_geodesic.mesh_surface_edge_point_path(
        mesh,
        edges=edges,
        positions=positions,
    )


def mesh_geodesic_edge_point_path(
    mesh: MeshDocument,
    *,
    start_point,
    edges,
    positions,
    end_point,
) -> dict[str, object]:
    return _rust_geodesic.mesh_geodesic_edge_point_path(
        mesh,
        start_point=start_point,
        edges=edges,
        positions=positions,
        end_point=end_point,
    )


def mesh_triangle_strip_unfolded_path(
    mesh: MeshDocument,
    *,
    start_face_index: int,
    crossed_edges,
    end_face_index: int,
    start_point,
    end_point,
) -> dict[str, object]:
    return _rust_geodesic.mesh_triangle_strip_unfolded_path(
        mesh,
        start_face_index=start_face_index,
        crossed_edges=crossed_edges,
        end_face_index=end_face_index,
        start_point=start_point,
        end_point=end_point,
    )


def mesh_steepest_descent_triangle_step(
    mesh: MeshDocument,
    *,
    vertex_scalars,
    face_index: int,
    start_barycentric,
) -> dict[str, object]:
    return _rust_geodesic.mesh_steepest_descent_triangle_step(
        mesh,
        vertex_scalars=vertex_scalars,
        face_index=face_index,
        start_barycentric=start_barycentric,
    )


def mesh_steepest_descent_edge_step(
    mesh: MeshDocument,
    *,
    vertex_scalars,
    edge,
    edge_position: float,
) -> dict[str, object]:
    return _rust_geodesic.mesh_steepest_descent_edge_step(
        mesh,
        vertex_scalars=vertex_scalars,
        edge=edge,
        edge_position=edge_position,
    )


def mesh_steepest_descent_vertex_step(
    mesh: MeshDocument,
    *,
    vertex_scalars,
    vertex_index: int,
) -> dict[str, object]:
    return _rust_geodesic.mesh_steepest_descent_vertex_step(
        mesh,
        vertex_scalars=vertex_scalars,
        vertex_index=vertex_index,
    )


def mesh_steepest_descent_path(
    mesh: MeshDocument,
    *,
    vertex_scalars,
    face_index: int,
    start_barycentric,
    max_steps: int = 1024,
) -> dict[str, object]:
    return _rust_geodesic.mesh_steepest_descent_path(
        mesh,
        vertex_scalars=vertex_scalars,
        face_index=face_index,
        start_barycentric=start_barycentric,
        max_steps=max_steps,
    )


def mesh_geodesic_distance_field(
    mesh: MeshDocument,
    *,
    seed_vertices,
    max_distance_mm: float | None = None,
) -> dict[str, object]:
    return _rust_geodesic.mesh_geodesic_distance_field(
        mesh,
        seed_vertices=seed_vertices,
        max_distance_mm=max_distance_mm,
    )


def mesh_closest_surface_path_targets(
    mesh: MeshDocument,
    *,
    start_vertices,
    end_vertices,
    max_distance_mm: float | None = None,
) -> dict[str, object]:
    return _rust_geodesic.mesh_closest_surface_path_targets(
        mesh,
        start_vertices=start_vertices,
        end_vertices=end_vertices,
        max_distance_mm=max_distance_mm,
    )


def mesh_surface_distance_seed_vertices(
    mesh: MeshDocument,
    *,
    seed_vertices=None,
    seed_edges=None,
    seed_face_ids=None,
) -> dict[str, object]:
    return _rust_geodesic.mesh_surface_distance_seed_vertices(
        mesh,
        seed_vertices=seed_vertices,
        seed_edges=seed_edges,
        seed_face_ids=seed_face_ids,
    )


def mesh_geodesic_iso_region(
    mesh: MeshDocument,
    *,
    seed_vertices,
    iso_value_mm: float,
    max_distance_mm: float | None = None,
) -> dict[str, object]:
    return _rust_geodesic.mesh_geodesic_iso_region(
        mesh,
        seed_vertices=seed_vertices,
        iso_value_mm=iso_value_mm,
        max_distance_mm=max_distance_mm,
    )


def mesh_geodesic_extreme_edges(
    mesh: MeshDocument,
    *,
    scalars,
    extreme_type: str = "ridge",
) -> dict[str, object]:
    return _rust_geodesic.mesh_geodesic_extreme_edges(
        mesh,
        scalars=scalars,
        extreme_type=extreme_type,
    )


def feature_pair_measurements(features, pairs) -> list[dict[str, object]]:
    return _rust_features.feature_pair_measurements(features, pairs)


def feature_object_descriptors(features, *, infinite_extent_mm: float = 1000.0) -> list[dict[str, object]]:
    return _rust_features.feature_object_descriptors(features, infinite_extent_mm=infinite_extent_mm)


def refine_feature_primitives(
    mesh: MeshDocument,
    features,
    *,
    distance_limit_mm: float = 0.1,
    normal_tolerance_degrees: float = 30.0,
    max_iterations: int = 10,
) -> list[dict[str, object]]:
    return _rust_features.refine_feature_primitives(
        mesh,
        features,
        distance_limit_mm=distance_limit_mm,
        normal_tolerance_degrees=normal_tolerance_degrees,
        max_iterations=max_iterations,
    )


def edge_face_map(mesh: MeshDocument) -> dict[tuple[int, int], list[int]]:
    return _rust_mesh.edge_face_map(mesh)


def extract_selected_faces_as_mesh(mesh: MeshDocument, selected_face_ids) -> MeshDocument:
    return _rust_mesh.extract_selected_faces_as_mesh(mesh, selected_face_ids)


def boundary_edges(mesh: MeshDocument) -> list[tuple[int, int]]:
    return _rust_mesh.boundary_edges_for_core(mesh)


def select_boundary_faces(mesh: MeshDocument) -> list[int]:
    return _rust_mesh.select_boundary_faces(mesh)


def select_boundary_edges(mesh: MeshDocument) -> list[tuple[int, int]]:
    return _rust_mesh.select_boundary_edges(mesh)


def bounded_seed_indices(mesh: MeshDocument, indices, max_count: int):
    return _rust_mesh_selection.bounded_seed_indices(mesh, indices, max_count)


def selection_seed_indices(
    mesh: MeshDocument,
    *,
    vertex_ids=None,
    face_ids=None,
    region_vertex_indices=None,
    brush_points_world=None,
):
    return _rust_mesh_selection.selection_seed_indices(
        mesh,
        vertex_ids=vertex_ids,
        face_ids=face_ids,
        region_vertex_indices=region_vertex_indices,
        brush_points_world=brush_points_world,
    )


def select_camera_facing_faces(
    mesh: MeshDocument,
    *,
    camera_direction,
    min_dot: float = 0.0,
) -> list[int]:
    return _rust_mesh.select_camera_facing_faces(
        mesh,
        camera_direction=camera_direction,
        min_dot=min_dot,
    )


def select_degenerate_faces(
    mesh: MeshDocument,
    *,
    min_aspect_ratio: float,
    boundary_only: bool = False,
) -> list[int]:
    selected = _rust_repair.select_degenerate_faces(
        mesh,
        min_aspect_ratio=min_aspect_ratio,
        boundary_only=boundary_only,
    )
    if selected is None:
        raise RuntimeError("Rust kernel select_degenerate_faces is required")
    return selected


def select_short_edges(mesh: MeshDocument, *, max_edge_length_mm: float) -> list[tuple[int, int]]:
    selected = _rust_repair.select_short_edges(mesh, max_edge_length_mm=max_edge_length_mm)
    if selected is None:
        raise RuntimeError("Rust kernel select_short_edges is required")
    return selected


def select_overhang_faces(
    mesh: MeshDocument,
    *,
    axis=(0.0, 0.0, 1.0),
    layer_height_mm: float,
    max_overhang_distance_mm: float,
    hops: int = 0,
) -> list[int]:
    return _rust_mesh.select_overhang_faces(
        mesh,
        axis=axis,
        layer_height_mm=layer_height_mm,
        max_overhang_distance_mm=max_overhang_distance_mm,
        hops=hops,
    )


def select_outer_layer_faces(mesh: MeshDocument, *, epsilon: float = 1e-8) -> list[int]:
    return _rust_mesh.select_outer_layer_faces(mesh, epsilon=epsilon)


def select_not_smooth_faces(mesh: MeshDocument, *, min_angle_radians: float = 0.3) -> list[int]:
    selected = _rust_repair.select_not_smooth_faces(mesh, min_angle_radians=min_angle_radians)
    if selected is None:
        raise RuntimeError("Rust kernel select_not_smooth_faces is required")
    return selected


def select_overlapping_faces(
    mesh: MeshDocument,
    *,
    max_dist_sq: float = 1e-10,
    max_normal_dot: float = -0.99,
    min_area_fraction: float = 1e-5,
) -> list[int]:
    return _rust_mesh.select_overlapping_faces(
        mesh,
        max_dist_sq=max_dist_sq,
        max_normal_dot=max_normal_dot,
        min_area_fraction=min_area_fraction,
    )


def graph_cut_select_region(
    mesh: MeshDocument,
    *,
    source_face_ids,
    sink_face_ids,
    boundary_weight: float = 1.0,
    curvature_preference: str = "geodesic",
) -> list[int]:
    return _rust_mesh.graph_cut_select_region(
        mesh,
        source_face_ids=source_face_ids,
        sink_face_ids=sink_face_ids,
        boundary_weight=boundary_weight,
        curvature_preference=curvature_preference,
    )


def graph_cut_select_region_auto_not_region(
    mesh: MeshDocument,
    *,
    source_face_ids,
    uncertainty_distance_mm: float,
    boundary_weight: float = 1.0,
    curvature_preference: str = "geodesic",
) -> list[int]:
    return _rust_mesh.graph_cut_select_region_auto_not_region(
        mesh,
        source_face_ids=source_face_ids,
        uncertainty_distance_mm=uncertainty_distance_mm,
        boundary_weight=boundary_weight,
        curvature_preference=curvature_preference,
    )


def select_faces_by_area(
    mesh: MeshDocument,
    *,
    area: float,
    scalar_type: str = "absolute",
    compare_type: str = "less",
) -> list[int]:
    return _rust_mesh.select_faces_by_area(
        mesh,
        area=area,
        scalar_type=scalar_type,
        compare_type=compare_type,
    )


def select_crease_edges(
    mesh: MeshDocument,
    *,
    angle_from_planar_radians: float = 3.0543261909900767,
    min_component_length_mm: float | None = None,
    min_branch_length_mm: float | None = None,
) -> list[tuple[int, int]]:
    report = _rust_smoothness.crease_edge_diagnostics(
        mesh,
        angle_from_planar_radians=angle_from_planar_radians,
        min_component_length_mm=min_component_length_mm,
        min_branch_length_mm=min_branch_length_mm,
    )
    if report is None:
        raise RuntimeError("Rust kernel crease_edge_diagnostics is required")
    return [entry.edge for entry in report.edges]


def face_adjacency(mesh: MeshDocument) -> list[list[int]]:
    return _rust_mesh.face_adjacency(mesh)


def connected_face_components(mesh: MeshDocument) -> list[list[int]]:
    return _rust_mesh.connected_face_components(mesh)


def select_largest_component_faces(mesh: MeshDocument, *, min_area_mm2: float = 0.0) -> list[int]:
    return _rust_mesh.select_largest_component_faces(mesh, min_area_mm2=min_area_mm2)


def expand_face_selection_to_components(mesh: MeshDocument, seed_face_ids: list[int]) -> list[int]:
    return _rust_mesh.expand_face_selection_to_components(mesh, seed_face_ids)


def apply_meshlib_selection_modifier(
    current_ids,
    incoming_ids,
    mode: str,
    *,
    item_count: int | None = None,
) -> list[int]:
    return _rust_mesh.apply_meshlib_selection_modifier(
        current_ids,
        incoming_ids,
        mode,
        item_count=item_count,
    )


def select_faces_by_screen_polygon(
    mesh: MeshDocument,
    view_projection_4x4,
    polygon_xy,
    *,
    include_backfaces: bool = True,
    visible_only: bool = False,
) -> list[int]:
    return _rust_mesh.select_faces_by_screen_polygon(
        mesh,
        view_projection_4x4,
        polygon_xy,
        include_backfaces=include_backfaces,
        visible_only=visible_only,
    )


def select_faces_by_screen_rect(
    mesh: MeshDocument,
    view_projection_4x4,
    rect_min_xy,
    rect_max_xy,
    *,
    include_backfaces: bool = True,
    visible_only: bool = False,
) -> list[int]:
    return _rust_mesh.select_faces_by_screen_rect(
        mesh,
        view_projection_4x4,
        rect_min_xy,
        rect_max_xy,
        include_backfaces=include_backfaces,
        visible_only=visible_only,
    )


def select_faces_by_screen_brush(
    mesh: MeshDocument,
    view_projection_4x4,
    brush_path_xy,
    *,
    radius_px: float,
    include_backfaces: bool = True,
    visible_only: bool = False,
) -> list[int]:
    return _rust_mesh.select_faces_by_screen_brush(
        mesh,
        view_projection_4x4,
        brush_path_xy,
        radius_px=radius_px,
        include_backfaces=include_backfaces,
        visible_only=visible_only,
    )


def select_face_by_ray(
    mesh: MeshDocument,
    ray_origin,
    ray_direction,
    *,
    epsilon: float = 1e-8,
    ignore_faces=None,
) -> list[int]:
    hit = _rust_spatial.first_ray_hit(
        mesh,
        ray_origin,
        ray_direction,
        epsilon=epsilon,
        ignore_faces=ignore_faces,
    )
    return [] if hit is None else [int(hit["face_index"])]


def select_inside_part_faces(mesh: MeshDocument) -> list[int]:
    return _rust_mesh.select_inside_part_faces(mesh)


def vertex_neighbors(mesh: MeshDocument) -> list[list[int]]:
    return _rust_mesh.vertex_neighbors(mesh)


def meshlib_object_mesh_scene_json(
    mesh: MeshDocument,
    *,
    object_name: str,
    child_index: int = 0,
    model_extension: str = ".ply",
) -> str:
    return _rust_mesh.meshlib_object_mesh_scene_json(
        mesh,
        object_name=object_name,
        child_index=child_index,
        model_extension=model_extension,
    )


def meshlib_object_mesh_scene_payload(
    mesh: MeshDocument,
    *,
    object_name: str,
    child_index: int = 0,
    model_extension: str = ".ply",
) -> dict[str, object]:
    return _rust_mesh.meshlib_object_mesh_scene_payload(
        mesh,
        object_name=object_name,
        child_index=child_index,
        model_extension=model_extension,
    )


def meshlib_object_mesh_mru_scene_bytes(
    mesh: MeshDocument,
    *,
    object_name: str,
    model_bytes: bytes,
    child_index: int = 0,
    model_extension: str = ".ply",
) -> bytes:
    return _rust_mesh.meshlib_object_mesh_mru_scene_bytes(
        mesh,
        object_name=object_name,
        model_bytes=model_bytes,
        child_index=child_index,
        model_extension=model_extension,
    )


def meshlib_multi_object_mru_scene_bytes(
    mesh: MeshDocument,
    *,
    root_name: str = "Root",
    root_key: str = "0_Root",
) -> bytes:
    return _rust_mesh.meshlib_multi_object_mru_scene_bytes(
        mesh,
        root_name=root_name,
        root_key=root_key,
    )


def meshlib_transform_scene_object(
    mesh: MeshDocument,
    *,
    object_key: str,
    xf: dict[str, object],
) -> MeshDocument:
    return _rust_mesh.meshlib_transform_scene_object(
        mesh,
        object_key=object_key,
        xf=xf,
    )


def meshlib_reparent_scene_object(
    mesh: MeshDocument,
    *,
    object_key: str,
    new_parent_key: str,
) -> MeshDocument:
    return _rust_mesh.meshlib_reparent_scene_object(
        mesh,
        object_key=object_key,
        new_parent_key=new_parent_key,
    )


def meshlib_set_scene_object_state(
    mesh: MeshDocument,
    *,
    object_key: str,
    visibility_mask: int | None = None,
    visible: bool | None = None,
    selected: bool | None = None,
    locked: bool | None = None,
    parent_locked: bool | None = None,
) -> MeshDocument:
    return _rust_mesh.meshlib_set_scene_object_state(
        mesh,
        object_key=object_key,
        visibility_mask=visibility_mask,
        visible=visible,
        selected=selected,
        locked=locked,
        parent_locked=parent_locked,
    )


def meshlib_select_scene_objects(
    mesh: MeshDocument,
    *,
    object_keys,
    mode: str = "select_one",
) -> MeshDocument:
    return _rust_mesh.meshlib_select_scene_objects(
        mesh,
        object_keys=object_keys,
        mode=mode,
    )


def meshlib_set_scene_feature_object_visualize_property(
    mesh: MeshDocument,
    *,
    object_key: str,
    property: str,
    viewport_mask: int,
    dimension_name: str | None = None,
) -> MeshDocument:
    return _rust_mesh.meshlib_set_scene_feature_object_visualize_property(
        mesh,
        object_key=object_key,
        property=property,
        viewport_mask=viewport_mask,
        dimension_name=dimension_name,
    )


def meshlib_scene_feature_object_render_payload(
    mesh: MeshDocument,
    *,
    viewport_mask: int = 0xFFFF_FFFF,
    circle_segments: int = 64,
) -> dict[str, object]:
    return _rust_mesh.meshlib_scene_feature_object_render_payload(
        mesh,
        viewport_mask=viewport_mask,
        circle_segments=circle_segments,
    )


def meshlib_reorder_scene_children(
    mesh: MeshDocument,
    *,
    parent_key: str,
    ordered_child_keys: list[str],
) -> MeshDocument:
    return _rust_mesh.meshlib_reorder_scene_children(
        mesh,
        parent_key=parent_key,
        ordered_child_keys=ordered_child_keys,
    )


def meshlib_apply_scene_ribbon_action(
    mesh: MeshDocument,
    *,
    action: str,
) -> MeshDocument:
    return _rust_mesh.meshlib_apply_scene_ribbon_action(
        mesh,
        action=action,
    )


def meshlib_group_scene_objects(
    mesh: MeshDocument,
    *,
    group_key: str,
) -> MeshDocument:
    return _rust_mesh.meshlib_group_scene_objects(
        mesh,
        group_key=group_key,
    )


def meshlib_ungroup_scene_objects(mesh: MeshDocument) -> MeshDocument:
    return _rust_mesh.meshlib_ungroup_scene_objects(mesh)


def meshlib_rename_scene_object(
    mesh: MeshDocument,
    *,
    object_key: str,
    object_name: str,
) -> MeshDocument:
    return _rust_mesh.meshlib_rename_scene_object(
        mesh,
        object_key=object_key,
        object_name=object_name,
    )


def mesh_from_mru_scene(source: bytes | bytearray) -> MeshDocument:
    return _rust_mesh.mesh_from_mru_scene(source)


def mesh_from_ply(source: bytes, texture_dir: str | Path | None = None) -> MeshDocument:
    return _rust_mesh.mesh_from_ply(source, texture_dir=texture_dir)


def mesh_from_obj(source: bytes | bytearray | str, material_dir: str | Path | None = None) -> MeshDocument:
    return _rust_mesh.mesh_from_obj(source, material_dir=material_dir)


def mesh_to_ply_bytes(mesh: MeshDocument) -> bytes:
    return _rust_mesh.mesh_to_ply_bytes(mesh)


def mesh_stats(mesh: MeshDocument) -> MeshStats:
    return _rust_stats.mesh_stats(mesh)
