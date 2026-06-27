#[pyfunction]
fn vertex_neighbors(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<Vec<i64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| zennah_geometry_core::vertex_neighbors_for_mesh(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn mesh_stats(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;

    let stats = py
        .detach(|| zennah_geometry_core::mesh_stats(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("bbox_min", stats.bbox_min.to_vec())?;
    output.set_item("bbox_max", stats.bbox_max.to_vec())?;
    output.set_item("bbox_size", stats.bbox_size.to_vec())?;
    output.set_item("surface_area_mm2", stats.surface_area_mm2)?;
    output.set_item("volume_mm3", stats.volume_mm3)?;
    output.set_item("vertex_count", stats.vertex_count)?;
    output.set_item("face_count", stats.face_count)?;
    output.set_item("connected_components", stats.connected_components)?;
    output.set_item("boundary_edge_count", stats.boundary_edge_count)?;
    Ok(output.unbind())
}

#[pyfunction]
fn boundary_loops(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<Vec<i64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let loops = py
        .detach(|| zennah_geometry_core::boundary_loops(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(loops
        .into_iter()
        .map(|component| component.into_iter().map(|value| value as i64).collect())
        .collect())
}

#[pyfunction]
fn mesh_health(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    detect_self_intersections: bool,
    max_self_intersection_faces: Option<usize>,
    epsilon: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let health = py
        .detach(|| {
            zennah_geometry_core::mesh_health(
                &rust_vertices,
                &rust_faces,
                detect_self_intersections,
                max_self_intersection_faces,
                epsilon,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let output = PyDict::new(py);
    output.set_item("is_closed", health.is_closed)?;
    output.set_item("holes_count", health.holes_count)?;
    output.set_item("boundary_edge_count", health.boundary_edge_count)?;
    output.set_item("nonmanifold_edge_count", health.nonmanifold_edge_count)?;
    output.set_item("self_intersections", health.self_intersections)?;
    output.set_item(
        "self_intersections_available",
        health.self_intersections_available,
    )?;
    Ok(output.unbind())
}

