mod analysis;
mod deform;
mod deform_smooth;
mod deform_target;
mod distance;
mod grid;
mod health_service;
mod hollow;
mod hollow_service;
mod jewelry;
mod manufacturability;
mod materials;
mod math;
mod mesh;
mod repair;
mod resize;
mod sdf_grid;
mod sdf_marching;
mod spatial;
mod topology;
mod types;
mod voxel;
mod voxel_mesh_ops;
pub use analysis::*;
pub use deform::{
    apply_brush_strokes, brush_stroke_weights, falloff_weights, laplacian_smooth_vertices,
    local_offset_vertices, outward_directions, region_brush_masks, smooth_vertices_with_falloff,
    weighted_laplacian_smooth_vertices,
};
pub use deform_smooth::taubin_smooth_vertices;
pub use deform_target::local_thicken_to_minimum_vertices;
pub use distance::nearest_distances_to_indices;
pub use health_service::service_mesh_health;
pub use hollow::{
    adaptive_hollow_to_weight, adaptive_protected_hollow_to_weight, drain_hole_cutter_mesh,
    drain_hole_cutters_mesh, inward_directions_for_hollow, plan_drain_holes, protected_hollow_mesh,
    protected_hollow_scale_field, weighted_inner_offset_vertices,
};
pub use hollow_service::{service_hollow_mesh, service_hollow_voxel_size};
pub use jewelry::{closest_ring_size, detect_ring_regions, measure_ring, ring_diameter_for_size};
pub use manufacturability::{
    build_recommendations, compute_manufacturability_report, health_score,
};
pub use materials::{
    grams_to_mm3, material_density_g_cm3, material_weight_table, mm3_to_grams, DEFAULT_MATERIAL,
    MATERIAL_DENSITIES_G_CM3,
};
pub use mesh::{
    boundary_edges_for_mesh, boundary_loops, connected_face_components_for_mesh,
    face_adjacency_for_mesh, face_normals_for_mesh, mesh_bounds, mesh_health, mesh_signed_volume,
    mesh_stats, mesh_surface_area, mesh_volume, normalize_axis_vector, ordered_edge_face_entries,
    safe_normalize_vector, safe_normalize_vectors, vertex_neighbors_for_mesh,
    vertex_normals_for_mesh, EdgeFaceEntry,
};
pub use repair::{
    basic_repair, fill_planar_holes, merge_close_vertices, ordered_boundary_loops,
    orient_faces_outward, remove_degenerate_faces, remove_unreferenced_vertices,
    repaired_surface_area, service_fill_holes,
};
pub use resize::{radial_scale_vertices, resize_ring_vertices};
pub use sdf_grid::{
    estimate_sdf_volume, sample_sdf_gradients_batch, sample_sdf_values_batch, sdf_cell_values,
    sdf_occupancy,
};
pub use sdf_marching::{
    finalized_marching_tetrahedra, finalized_sdf_boolean_marching_tetrahedra,
    finalized_sdf_offset_marching_tetrahedra, finalized_sdf_shell_marching_tetrahedra,
};
pub use spatial::*;
pub use topology::orient_faces_consistently;
pub use types::*;
pub use voxel::{
    extract_surface_mesh_from_sdf_cells, marching_tetrahedra, project_vertices_to_sdf,
    refine_vertices_with_sdf, sdf_boolean_marching_tetrahedra, sdf_boolean_values,
    sdf_offset_marching_tetrahedra, sdf_offset_values, sdf_shell_marching_tetrahedra,
    sdf_shell_values,
};
pub use voxel_mesh_ops::*;
#[cfg(test)]
mod tests;
