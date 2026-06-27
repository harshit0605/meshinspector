"""Rust accelerator facade with implementation in private `_rust_*` modules."""
from __future__ import annotations
from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.accelerators._rust_analysis import (
    compare_summary,
    nearest_surface_distances,
    nearest_vertex_distances,
    scalar_overlay_payload,
    section_contour,
    service_compare_distances,
    service_compare_summary,
    signed_compare_summary,
    signed_surface_distances,
    summarize_thickness,
    version_compare_distances,
    version_compare_summary,
)
from geometry_sdk.accelerators._rust_aabb_tree import (
    closest_candidate_faces,
    overlapping_face_pairs,
    point_aabb_distance_sq,
    ray_candidate_faces,
    ray_intersects_aabb,
)
from geometry_sdk.accelerators._rust_common import accelerator_mode, available, backend_name
from geometry_sdk.accelerators._rust_deform import (
    apply_brush_strokes,
    falloff_weights,
    laplacian_smooth_vertices,
    local_thicken_to_minimum_vertices,
    local_offset_vertices,
    outward_directions,
    smooth_vertices_with_falloff,
    taubin_smooth_vertices,
    weighted_laplacian_smooth_vertices,
)
from geometry_sdk.accelerators._rust_distance import distance_map_contour_boolean, distance_map_from_contours, distance_map_from_mesh, distance_map_from_tiff, distance_map_merge, distance_map_to_tiff, distance_map_to_iso_segments, nearest_distances
from geometry_sdk.accelerators._rust_fast_marching import (
    mesh_fast_marching_surface_path,
    mesh_fast_marching_surface_path_tri_points,
    mesh_surface_path_tri_points,
)
from geometry_sdk.accelerators._rust_gcode import gcode_machine_settings_from_meshlib_json, gcode_machine_settings_to_meshlib_json, load_gcode_source, parse_gcode_file_paths, parse_gcode_paths, write_gcode_source; from geometry_sdk.accelerators._rust_lines import object_lines_from_contours, object_lines_from_mrlines, object_lines_from_ply, object_lines_from_pts, object_lines_from_svg, object_lines_to_contours, object_lines_to_dxf, object_lines_to_mrlines, object_lines_to_ply, object_lines_to_pts, offset_contours, offset_contours_variable, offset_contours_variable_with_origins, offset_contours_with_origins
from geometry_sdk.accelerators._rust_geodesic import mesh_closest_surface_path_targets, mesh_cut_measure_contours, mesh_cut_measure_edge_path_topology_cut, mesh_geodesic_distance_field, mesh_geodesic_edge_point_path, mesh_geodesic_extreme_edges, mesh_geodesic_iso_region, mesh_geodesic_path, mesh_geodesic_polyline_path, mesh_geodesic_quadrangle_path, mesh_planar_triangle_strip_path, mesh_surface_edge_point_path, mesh_surface_distance_seed_vertices, mesh_steepest_descent_edge_step, mesh_steepest_descent_path, mesh_steepest_descent_triangle_step, mesh_steepest_descent_vertex_step, mesh_triangle_strip_unfolded_path
from geometry_sdk.accelerators._rust_hollow import (
    adaptive_hollow_to_weight,
    adaptive_protected_hollow_to_weight,
    drain_hole_cutter_mesh,
    drain_hole_cutters_mesh,
    inward_directions_for_hollow,
    plan_drain_holes,
    protected_hollow_mesh,
    protected_hollow_scale_field,
    service_hollow_mesh,
    service_hollow_voxel_size,
    weighted_inner_offset_preview,
)
from geometry_sdk.accelerators._rust_holes import hole_complicating_faces_diagnostics, remove_hole_complicating_faces, repeated_hole_boundary_vertices_diagnostics
from geometry_sdk.accelerators._rust_jewelry import (
    closest_ring_size,
    detect_ring_regions,
    measure_ring,
    ring_diameter_for_size,
)
from geometry_sdk.accelerators._rust_materials import (
    grams_to_mm3,
    material_densities_g_cm3,
    material_weight_table,
    mm3_to_grams,
)
from geometry_sdk.accelerators._rust_manufacturability import (
    build_recommendations,
    compute_manufacturability_report,
    health_score,
)
from geometry_sdk.accelerators._rust_health import service_mesh_health
from geometry_sdk.accelerators._rust_mesh import (
    boundary_loops,
    expand_face_selection_to_components,
    graph_cut_select_region,
    graph_cut_select_region_auto_not_region,
    mesh_from_obj,
    mesh_from_ply,
    mesh_health,
    mesh_stats,
    select_boundary_edges,
    select_boundary_faces,
    select_camera_facing_faces,
    select_faces_by_area,
    select_faces_by_screen_brush,
    select_faces_by_screen_polygon,
    select_faces_by_screen_rect,
    select_inside_part_faces,
    select_largest_component_faces,
    select_outer_layer_faces,
    select_overlapping_faces,
    select_overhang_faces,
)
from geometry_sdk.accelerators._rust_point_cloud import (
    point_cloud_grid_sample_indices,
    point_cloud_extract_selected_points_as_object,
    point_cloud_local_fan_triangles,
    point_cloud_local_neighbor_fan,
    point_cloud_local_triangulation_repetitions,
    point_cloud_n_closest_neighbors,
    point_cloud_nearest_projections,
    point_cloud_neighbors_in_radius,
    point_cloud_pick_by_ray,
    point_cloud_project_to_mesh,
    point_cloud_select_by_screen_brush,
    point_cloud_select_by_screen_polygon,
    point_cloud_select_by_screen_rect,
    point_cloud_triangulate_candidate_mesh,
    point_cloud_triangulate_cleaned_candidate_mesh,
    point_cloud_triangulate_filled_candidate_mesh,
    point_cloud_triangulate_topology_candidate_mesh,
    point_cloud_two_closest_points,
    point_cloud_uniform_sample_indices,
)
from geometry_sdk.accelerators._rust_point_cloud_io import point_cloud_from_ply, point_cloud_to_ply
from geometry_sdk.accelerators._rust_point_cloud_registration import multiway_aabb_cascade_combined_icp, multiway_aabb_cascade_point_to_plane_icp, multiway_aabb_cascade_point_to_point_icp, multiway_all_object_combined_icp, multiway_all_object_point_to_plane_icp, multiway_all_object_point_to_point_icp, multiway_combined_icp, multiway_point_to_plane_icp, multiway_point_to_point_icp, multiway_sequential_cascade_combined_icp, multiway_sequential_cascade_point_to_plane_icp, multiway_sequential_cascade_point_to_point_icp, pairwise_point_to_plane_icp, pairwise_point_to_point_icp
from geometry_sdk.accelerators._rust_repair import (
    basic_repair,
    degenerate_face_diagnostics,
    duplicate_multi_hole_vertices,
    fill_planar_holes,
    find_disoriented_faces,
    flip_normals,
    fix_self_intersections_relax,
    hole_fill_plan_diagnostics,
    mesh_healer_diagnostics,
    multiple_edge_diagnostics,
    not_smooth_face_diagnostics,
    ordered_boundary_loops,
    orient_faces_outward,
    prune_small_components,
    weld_coincident_vertices,
    rebuild_via_sdf,
    repair_multiple_edges,
    remove_degenerate_faces,
    repaired_surface_area,
    select_degenerate_faces,
    select_not_smooth_faces,
    select_short_edges,
    service_fill_holes,
    short_edge_diagnostics,
)
from geometry_sdk.accelerators._rust_vertex_repair import (
    merge_close_vertices,
    remove_unreferenced_vertices,
    unite_close_vertices,
)
from geometry_sdk.accelerators._rust_resize import fit_ring_to_diameter_vertices, radial_scale_vertices, resize_ring_vertices; from geometry_sdk.accelerators._rust_intersections import exact_mesh_intersections
from geometry_sdk.accelerators._rust_mesh_quality import boolean_output_failures, decimate_output_failures, hollow_output_failures, offset_shell_failures
from geometry_sdk.accelerators._rust_spatial import (
    closest_points_on_mesh,
    first_ray_hit,
    first_ray_hits,
    point_mesh_distances,
    ray_thickness_at_vertices,
    sdf_grid_values,
    self_intersecting_faces,
    signed_point_mesh_distances,
    winding_numbers,
)
from geometry_sdk.accelerators._rust_smoothness import crease_edge_diagnostics, crease_repair_plan_diagnostics, fix_mesh_creases
from geometry_sdk.accelerators._rust_thickness import insphere_thickness_at_vertices, service_thickness_at_vertices
from geometry_sdk.accelerators._rust_topology import orient_faces_consistently; from geometry_sdk.accelerators._rust_tunnels import detect_tunnel_faces, eliminate_tunnels, tunnel_diagnostics; from geometry_sdk.accelerators._rust_nonmanifold import duplicate_nonmanifold_vertices, repair_nonmanifold_edges
from geometry_sdk.accelerators._rust_voxel import (
    load_raw_voxels,
    load_raw_voxels_auto,
    load_tiff_voxels_dir,
    marching_tetrahedra,
    meshlib_vdb_payload_to_dual_mesh,
    project_vertices_to_sdf,
    refine_vertices_with_sdf,
    sdf_boolean_marching_tetrahedra,
    sdf_boolean_values,
    sdf_offset_marching_tetrahedra,
    sdf_shell_marching_tetrahedra,
    voxel_active_box_values,
    voxel_default_iso_value,
    voxel_line_graph_values,
    voxel_path_build_four_values,
    voxel_path_values,
    voxel_mask_to_mesh_values,
    voxel_move_mesh_to_max_deriv_values,
    voxel_segmentation_mesh_values,
    voxel_segmentation_values,
    voxel_slice_values,
    voxel_to_mesh_dual_values,
    voxel_to_mesh_simple_values,
    voxel_volume_render_data_values,
    voxel_volume_render_lut_values,
    voxel_volume_render_ray_values,
)
_rs = _common._rs; __all__ = ["_rs", *(name for name in globals() if not name.startswith("_"))]
