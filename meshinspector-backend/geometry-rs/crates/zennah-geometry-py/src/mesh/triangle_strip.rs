#[pyfunction]
fn mesh_triangle_strip_unfolded_path(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    start_face_index: i64,
    crossed_edges: PyReadonlyArray2<'_, i64>,
    end_face_index: i64,
    start_point: PyReadonlyArray1<'_, f64>,
    end_point: PyReadonlyArray1<'_, f64>,
) -> PyResult<Py<PyDict>> {
    if start_face_index < 0 || end_face_index < 0 {
        return Err(PyValueError::new_err(
            "start_face_index and end_face_index must be non-negative",
        ));
    }
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_edges = read_edges2(crossed_edges)?;
    let rust_start = read_vec3("start_point", start_point)?;
    let rust_end = read_vec3("end_point", end_point)?;
    let path = py
        .detach(|| {
            zennah_geometry_core::mesh_triangle_strip_unfolded_path(
                &rust_vertices,
                &rust_faces,
                start_face_index as usize,
                &rust_edges,
                end_face_index as usize,
                rust_start,
                rust_end,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("start_face_index", path.start_face_index)?;
    output.set_item("end_face_index", path.end_face_index)?;
    output.set_item(
        "strip_face_indices",
        path.strip_face_indices
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "crossed_edges",
        path.crossed_edges
            .into_iter()
            .map(|edge| edge.into_iter().map(|index| index as i64).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "oriented_edges",
        path.oriented_edges
            .into_iter()
            .map(|edge| edge.into_iter().map(|index| index as i64).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("crossing_positions", path.crossing_positions)?;
    output.set_item("crossing_points", vec3_lists(path.crossing_points))?;
    output.set_item("points", vec3_lists(path.points))?;
    output.set_item("segment_lengths", path.segment_lengths)?;
    output.set_item("length_mm", path.length_mm)?;
    output.set_item("planar_length_mm", path.planar_length_mm)?;
    output.set_item("meshlib_reference", path.meshlib_reference)?;
    Ok(output.unbind())
}
