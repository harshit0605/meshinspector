pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(remove_degenerate_faces, module)?)?;
    module.add_function(wrap_pyfunction!(remove_unreferenced_vertices, module)?)?;
    module.add_function(wrap_pyfunction!(merge_close_vertices, module)?)?;
    module.add_function(wrap_pyfunction!(unite_close_vertices, module)?)?;
    module.add_function(wrap_pyfunction!(orient_faces_outward, module)?)?;
    module.add_function(wrap_pyfunction!(flip_normals, module)?)?;
    module.add_function(wrap_pyfunction!(find_disoriented_faces, module)?)?;
    module.add_function(wrap_pyfunction!(basic_repair, module)?)?;
    module.add_function(wrap_pyfunction!(fix_self_intersections_relax, module)?)?;
    module.add_function(wrap_pyfunction!(repaired_surface_area, module)?)?;
    module.add_function(wrap_pyfunction!(ordered_boundary_loops, module)?)?;
    module.add_function(wrap_pyfunction!(rebuild_via_sdf, module)?)?;
    Ok(())
}
