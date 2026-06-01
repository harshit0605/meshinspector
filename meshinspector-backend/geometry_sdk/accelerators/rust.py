"""Rust accelerator facade.

The Rust extension is optional. SDK modules call this boundary in `auto` mode
and fall back to Python when the extension is not installed. Keep this module as
an import-stable facade; implementation lives in private `_rust_*` modules.
"""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.accelerators._rust_analysis import (
    compare_summary,
    nearest_surface_distances,
    nearest_vertex_distances,
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
from geometry_sdk.accelerators._rust_distance import nearest_distances
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
from geometry_sdk.accelerators._rust_mesh import boundary_loops, mesh_health, mesh_stats
from geometry_sdk.accelerators._rust_repair import (
    basic_repair,
    fill_planar_holes,
    merge_close_vertices,
    ordered_boundary_loops,
    orient_faces_outward,
    rebuild_via_sdf,
    remove_degenerate_faces,
    remove_unreferenced_vertices,
    repaired_surface_area,
    service_fill_holes,
)
from geometry_sdk.accelerators._rust_resize import radial_scale_vertices, resize_ring_vertices
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
from geometry_sdk.accelerators._rust_thickness import (
    insphere_thickness_at_vertices,
    service_thickness_at_vertices,
)
from geometry_sdk.accelerators._rust_topology import orient_faces_consistently
from geometry_sdk.accelerators._rust_voxel import (
    marching_tetrahedra,
    project_vertices_to_sdf,
    refine_vertices_with_sdf,
    sdf_boolean_marching_tetrahedra,
    sdf_boolean_values,
    sdf_offset_marching_tetrahedra,
    sdf_shell_marching_tetrahedra,
)

# Kept for diagnostics/backwards compatibility. Tests and internals should patch
# `_rust_common._rs`, because implementation modules read that shared handle.
_rs = _common._rs

__all__ = ["_rs", *(name for name in globals() if not name.startswith("_"))]
