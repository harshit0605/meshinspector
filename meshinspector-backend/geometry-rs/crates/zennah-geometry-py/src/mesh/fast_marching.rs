#[pyfunction]
fn mesh_fast_marching_surface_path(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    start_vertex: usize,
    end_vertex: usize,
    max_steps: usize,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let path = py
        .detach(|| {
            zennah_geometry_core::mesh_fast_marching_surface_path(
                &rust_vertices,
                &rust_faces,
                start_vertex,
                end_vertex,
                max_steps,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("start_vertex", path.start_vertex)?;
    output.set_item("end_vertex", path.end_vertex)?;
    output.set_item("start_face_index", path.start_face_index)?;
    output.set_item("start_barycentric", path.start_barycentric.to_vec())?;
    output.set_item("surface_distances_mm", path.surface_distances_mm)?;
    output.set_item(
        "surface_predecessor_vertices",
        path.surface_predecessor_vertices
            .into_iter()
            .map(|index| index.map_or(-1_i64, |value| value as i64))
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "edges",
        path.edges
            .into_iter()
            .map(|edge| {
                edge.into_iter()
                    .map(|index| index as i64)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
    )?;
    output.set_item("positions", path.positions)?;
    output.set_item("points", vec3_lists(path.points))?;
    output.set_item("segment_lengths", path.segment_lengths)?;
    output.set_item("length_mm", path.length_mm)?;
    output.set_item(
        "reached_vertex",
        path.reached_vertex.map(|vertex| vertex as i64),
    )?;
    output.set_item("stopped_reason", path.stopped_reason)?;
    output.set_item("steps", path.steps)?;
    output.set_item("meshlib_reference", path.meshlib_reference)?;
    Ok(output.unbind())
}

#[pyfunction]
fn mesh_fast_marching_surface_path_tri_points(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    start_face_index: usize,
    start_barycentric: [f64; 3],
    end_face_index: usize,
    end_barycentric: [f64; 3],
    max_steps: usize,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let path = py
        .detach(|| {
            zennah_geometry_core::mesh_fast_marching_surface_path_tri_points(
                &rust_vertices,
                &rust_faces,
                start_face_index,
                start_barycentric,
                end_face_index,
                end_barycentric,
                max_steps,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("start_face_index", path.start_face_index)?;
    output.set_item("start_barycentric", path.start_barycentric.to_vec())?;
    output.set_item("start_point", path.start_point.to_vec())?;
    output.set_item("end_face_index", path.end_face_index)?;
    output.set_item("end_barycentric", path.end_barycentric.to_vec())?;
    output.set_item("end_point", path.end_point.to_vec())?;
    output.set_item("surface_distances_mm", path.surface_distances_mm)?;
    output.set_item(
        "surface_predecessor_vertices",
        path.surface_predecessor_vertices
            .into_iter()
            .map(|index| index.map_or(-1_i64, |value| value as i64))
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "edges",
        path.edges
            .into_iter()
            .map(|edge| {
                edge.into_iter()
                    .map(|index| index as i64)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
    )?;
    output.set_item("positions", path.positions)?;
    output.set_item("points", vec3_lists(path.points))?;
    output.set_item("segment_lengths", path.segment_lengths)?;
    output.set_item("length_mm", path.length_mm)?;
    output.set_item(
        "reached_face_index",
        path.reached_face_index.map(|face| face as i64),
    )?;
    output.set_item("stopped_reason", path.stopped_reason)?;
    output.set_item("steps", path.steps)?;
    output.set_item("meshlib_reference", path.meshlib_reference)?;
    Ok(output.unbind())
}

#[pyfunction]
fn mesh_surface_path_tri_points(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    start_face_index: usize,
    start_barycentric: [f64; 3],
    end_face_index: usize,
    end_barycentric: [f64; 3],
    max_geodesic_iters: usize,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let path = py
        .detach(|| {
            zennah_geometry_core::mesh_surface_path_tri_points(
                &rust_vertices,
                &rust_faces,
                start_face_index,
                start_barycentric,
                end_face_index,
                end_barycentric,
                max_geodesic_iters,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("start_face_index", path.start_face_index)?;
    output.set_item("start_barycentric", path.start_barycentric.to_vec())?;
    output.set_item("start_point", path.start_point.to_vec())?;
    output.set_item("end_face_index", path.end_face_index)?;
    output.set_item("end_barycentric", path.end_barycentric.to_vec())?;
    output.set_item("end_point", path.end_point.to_vec())?;
    output.set_item("surface_distances_mm", path.surface_distances_mm)?;
    output.set_item(
        "surface_predecessor_vertices",
        path.surface_predecessor_vertices
            .into_iter()
            .map(|index| index.map_or(-1_i64, |value| value as i64))
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "approximate_edges",
        path.approximate_edges
            .into_iter()
            .map(|edge| {
                edge.into_iter()
                    .map(|index| index as i64)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
    )?;
    output.set_item("approximate_positions", path.approximate_positions)?;
    output.set_item("approximate_points", vec3_lists(path.approximate_points))?;
    output.set_item(
        "edges",
        path.edges
            .into_iter()
            .map(|edge| {
                edge.into_iter()
                    .map(|index| index as i64)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
    )?;
    output.set_item("positions", path.positions)?;
    output.set_item("points", vec3_lists(path.points))?;
    output.set_item("segment_lengths", path.segment_lengths)?;
    output.set_item("length_mm", path.length_mm)?;
    output.set_item(
        "reached_face_index",
        path.reached_face_index.map(|face| face as i64),
    )?;
    output.set_item("stopped_reason", path.stopped_reason)?;
    output.set_item("reduce_iterations", path.reduce_iterations)?;
    output.set_item("steps", path.steps)?;
    output.set_item("meshlib_reference", path.meshlib_reference)?;
    Ok(output.unbind())
}
