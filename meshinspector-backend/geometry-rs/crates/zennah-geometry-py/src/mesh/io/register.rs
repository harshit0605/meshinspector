pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(safe_normalize_vector, module)?)?;
    module.add_function(wrap_pyfunction!(safe_normalize_vectors, module)?)?;
    module.add_function(wrap_pyfunction!(normalize_axis, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_bounds, module)?)?;
    module.add_function(wrap_pyfunction!(face_normals, module)?)?;
    module.add_function(wrap_pyfunction!(vertex_normals, module)?)?;
    module.add_function(wrap_pyfunction!(surface_area, module)?)?;
    module.add_function(wrap_pyfunction!(signed_volume, module)?)?;
    module.add_function(wrap_pyfunction!(volume, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_geodesic_path, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_fast_marching_surface_path, module)?)?;
    module.add_function(wrap_pyfunction!(
        mesh_fast_marching_surface_path_tri_points,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(mesh_surface_path_tri_points, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_geodesic_polyline_path, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_cut_measure_contours, module)?)?;
    module.add_function(wrap_pyfunction!(
        mesh_cut_measure_edge_path_topology_cut,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(mesh_geodesic_quadrangle_path, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_planar_triangle_strip_path, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_surface_edge_point_path, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_geodesic_edge_point_path, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_triangle_strip_unfolded_path, module)?)?;
    module.add_function(wrap_pyfunction!(
        mesh_steepest_descent_triangle_step,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        mesh_steepest_descent_edge_step,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        mesh_steepest_descent_vertex_step,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(mesh_steepest_descent_path, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_geodesic_distance_field, module)?)?;
    module.add_function(wrap_pyfunction!(
        mesh_closest_surface_path_targets,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        mesh_surface_distance_seed_vertices,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(mesh_geodesic_iso_region, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_geodesic_extreme_edges, module)?)?;
    module.add_function(wrap_pyfunction!(edge_face_map, module)?)?;
    module.add_function(wrap_pyfunction!(extract_selected_faces_as_mesh, module)?)?;
    module.add_function(wrap_pyfunction!(boundary_edges, module)?)?;
    module.add_function(wrap_pyfunction!(select_boundary_faces, module)?)?;
    module.add_function(wrap_pyfunction!(select_boundary_edges, module)?)?;
    module.add_function(wrap_pyfunction!(bounded_seed_indices, module)?)?;
    module.add_function(wrap_pyfunction!(selection_seed_indices, module)?)?;
    module.add_function(wrap_pyfunction!(select_camera_facing_faces, module)?)?;
    module.add_function(wrap_pyfunction!(select_overhang_faces, module)?)?;
    module.add_function(wrap_pyfunction!(select_outer_layer_faces, module)?)?;
    module.add_function(wrap_pyfunction!(graph_cut_select_region, module)?)?;
    module.add_function(wrap_pyfunction!(
        graph_cut_select_region_auto_not_region,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(select_overlapping_faces, module)?)?;
    module.add_function(wrap_pyfunction!(select_faces_by_area, module)?)?;
    module.add_function(wrap_pyfunction!(face_adjacency, module)?)?;
    module.add_function(wrap_pyfunction!(connected_face_components, module)?)?;
    module.add_function(wrap_pyfunction!(select_largest_component_faces, module)?)?;
    module.add_function(wrap_pyfunction!(
        expand_face_selection_to_components,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(apply_meshlib_selection_modifier, module)?)?;
    module.add_function(wrap_pyfunction!(select_faces_by_screen_polygon, module)?)?;
    module.add_function(wrap_pyfunction!(select_faces_by_screen_rect, module)?)?;
    module.add_function(wrap_pyfunction!(select_faces_by_screen_brush, module)?)?;
    module.add_function(wrap_pyfunction!(select_inside_part_faces, module)?)?;
    module.add_function(wrap_pyfunction!(vertex_neighbors, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_stats, module)?)?;
    module.add_function(wrap_pyfunction!(boundary_loops, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_health, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_from_ply, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_to_ply, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_from_obj, module)?)?;
    module.add_function(wrap_pyfunction!(mesh_from_mru_scene, module)?)?;
    module.add_function(wrap_pyfunction!(meshlib_object_mesh_scene_payload, module)?)?;
    module.add_function(wrap_pyfunction!(meshlib_object_mesh_mru_scene, module)?)?;
    module.add_function(wrap_pyfunction!(meshlib_multi_object_mru_scene, module)?)?;
    module.add_function(wrap_pyfunction!(meshlib_transform_scene_object, module)?)?;
    module.add_function(wrap_pyfunction!(meshlib_reparent_scene_object, module)?)?;
    module.add_function(wrap_pyfunction!(meshlib_set_scene_object_state, module)?)?;
    module.add_function(wrap_pyfunction!(meshlib_select_scene_objects, module)?)?;
    module.add_function(wrap_pyfunction!(
        meshlib_set_scene_feature_object_visualize_property,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        meshlib_scene_feature_object_render_payload,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(meshlib_reorder_scene_children, module)?)?;
    module.add_function(wrap_pyfunction!(meshlib_apply_scene_ribbon_action, module)?)?;
    module.add_function(wrap_pyfunction!(meshlib_rename_scene_object, module)?)?;
    module.add_function(wrap_pyfunction!(meshlib_group_scene_objects, module)?)?;
    module.add_function(wrap_pyfunction!(meshlib_ungroup_scene_objects, module)?)?;
    Ok(())
}
