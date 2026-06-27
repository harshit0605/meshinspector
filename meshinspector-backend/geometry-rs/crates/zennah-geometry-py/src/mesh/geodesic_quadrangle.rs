#[pyfunction]
fn mesh_geodesic_quadrangle_path(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    start_vertex: i64,
    end_vertex: i64,
) -> PyResult<Py<PyDict>> {
    if start_vertex < 0 || end_vertex < 0 {
        return Err(PyValueError::new_err(
            "start_vertex and end_vertex must be non-negative",
        ));
    }
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let path = py
        .detach(|| {
            zennah_geometry_core::mesh_geodesic_quadrangle_path(
                &rust_vertices,
                &rust_faces,
                start_vertex as usize,
                end_vertex as usize,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("start_vertex", path.start_vertex)?;
    output.set_item("end_vertex", path.end_vertex)?;
    output.set_item("start_face_index", path.start_face_index)?;
    output.set_item("end_face_index", path.end_face_index)?;
    output.set_item(
        "shared_edge",
        vec![path.shared_edge[0] as i64, path.shared_edge[1] as i64],
    )?;
    output.set_item("crossing_t", path.crossing_t)?;
    output.set_item("crossing_point", path.crossing_point.to_vec())?;
    output.set_item("points", vec3_lists(path.points))?;
    output.set_item("edge_lengths", path.edge_lengths)?;
    output.set_item("length_mm", path.length_mm)?;
    output.set_item(
        "graph_vertex_indices",
        path.graph_vertex_indices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item("graph_length_mm", path.graph_length_mm)?;
    output.set_item("unfolded_quadrangle_convex", path.unfolded_quadrangle_convex)?;
    output.set_item("meshlib_reference", path.meshlib_reference)?;
    Ok(output.unbind())
}
