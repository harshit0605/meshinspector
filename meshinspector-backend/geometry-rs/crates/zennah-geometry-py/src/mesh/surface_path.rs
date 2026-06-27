#[pyfunction]
fn mesh_surface_edge_point_path(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    edges: PyReadonlyArray2<'_, i64>,
    positions: PyReadonlyArray1<'_, f64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_edges = read_edges2(edges)?;
    let rust_positions = read_f64_values(positions);
    let path = py
        .detach(|| {
            zennah_geometry_core::mesh_surface_edge_point_path(
                &rust_vertices,
                &rust_faces,
                &rust_edges,
                &rust_positions,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item(
        "edges",
        path.edges
            .into_iter()
            .map(|edge| edge.into_iter().map(|index| index as i64).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("positions", path.positions)?;
    output.set_item("points", vec3_lists(path.points))?;
    output.set_item("segment_lengths", path.segment_lengths)?;
    output.set_item("length_mm", path.length_mm)?;
    output.set_item("meshlib_reference", path.meshlib_reference)?;
    Ok(output.unbind())
}

#[pyfunction]
fn mesh_geodesic_edge_point_path(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    start_point: PyReadonlyArray1<'_, f64>,
    edges: PyReadonlyArray2<'_, i64>,
    positions: PyReadonlyArray1<'_, f64>,
    end_point: PyReadonlyArray1<'_, f64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_start = read_vec3("start_point", start_point)?;
    let rust_edges = read_edges2(edges)?;
    let rust_positions = read_f64_values(positions);
    let rust_end = read_vec3("end_point", end_point)?;
    let path = py
        .detach(|| {
            zennah_geometry_core::mesh_geodesic_edge_point_path(
                &rust_vertices,
                &rust_faces,
                rust_start,
                &rust_edges,
                &rust_positions,
                rust_end,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("start_point", path.start_point.to_vec())?;
    output.set_item("end_point", path.end_point.to_vec())?;
    output.set_item(
        "edges",
        path.edges
            .into_iter()
            .map(|edge| edge.into_iter().map(|index| index as i64).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("positions", path.positions)?;
    output.set_item("mid_points", vec3_lists(path.mid_points))?;
    output.set_item("points", vec3_lists(path.points))?;
    output.set_item("segment_lengths", path.segment_lengths)?;
    output.set_item("length_mm", path.length_mm)?;
    output.set_item("meshlib_reference", path.meshlib_reference)?;
    Ok(output.unbind())
}

fn read_edges2(values: PyReadonlyArray2<'_, i64>) -> PyResult<Vec<[i64; 2]>> {
    let rows = values.as_array();
    if rows.ndim() != 2 || rows.shape()[1] != 2 {
        return Err(PyValueError::new_err("edges must have shape (n, 2)"));
    }
    let mut edges = Vec::with_capacity(rows.shape()[0]);
    for row in rows.outer_iter() {
        edges.push([row[0], row[1]]);
    }
    Ok(edges)
}
