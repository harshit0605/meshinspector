#[pyfunction(signature = (vertices, faces, critical_length_mm))]
fn short_edge_diagnostics(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
    critical_length_mm: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::short_edge_diagnostics(
                &rust_vertices,
                &rust_faces,
                critical_length_mm,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    short_edge_report_dict(py, result)
}

#[pyfunction(signature = (vertices, faces, max_edge_length_mm))]
fn select_short_edges(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
    max_edge_length_mm: f64,
) -> PyResult<Vec<Vec<i64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let edges = py
        .detach(|| {
            zennah_geometry_core::select_short_edges(
                &rust_vertices,
                &rust_faces,
                max_edge_length_mm,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(edges.into_iter().map(|edge| edge.to_vec()).collect())
}

#[pyfunction(signature = (vertices, faces, critical_aspect_ratio))]
fn degenerate_face_diagnostics(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
    critical_aspect_ratio: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::degenerate_face_diagnostics(
                &rust_vertices,
                &rust_faces,
                critical_aspect_ratio,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    degenerate_face_report_dict(py, result)
}

#[pyfunction(signature = (vertices, faces, min_aspect_ratio, boundary_only=false))]
fn select_degenerate_faces(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
    min_aspect_ratio: f64,
    boundary_only: bool,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| {
        zennah_geometry_core::select_degenerate_faces(
            &rust_vertices,
            &rust_faces,
            min_aspect_ratio,
            boundary_only,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction(signature = (vertices, faces))]
fn multiple_edge_diagnostics(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| zennah_geometry_core::multiple_edge_diagnostics(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    multiple_edge_report_dict(py, result)
}

#[pyfunction(signature = (vertices, faces))]
fn repair_multiple_edges(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| zennah_geometry_core::repair_multiple_edges(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    multiple_edge_repair_result_dict(py, result)
}

#[pyfunction(signature = (vertices, faces))]
fn duplicate_multi_hole_vertices(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| zennah_geometry_core::duplicate_multi_hole_vertices(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    duplicate_multi_hole_vertices_result_dict(py, result)
}

#[pyfunction(signature = (vertices, faces))]
fn repair_nonmanifold_edges(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| zennah_geometry_core::repair_nonmanifold_edges(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    nonmanifold_edge_repair_result_dict(py, result)
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(short_edge_diagnostics, module)?)?;
    module.add_function(wrap_pyfunction!(select_short_edges, module)?)?;
    module.add_function(wrap_pyfunction!(degenerate_face_diagnostics, module)?)?;
    module.add_function(wrap_pyfunction!(select_degenerate_faces, module)?)?;
    module.add_function(wrap_pyfunction!(multiple_edge_diagnostics, module)?)?;
    module.add_function(wrap_pyfunction!(repair_multiple_edges, module)?)?;
    module.add_function(wrap_pyfunction!(duplicate_multi_hole_vertices, module)?)?;
    module.add_function(wrap_pyfunction!(repair_nonmanifold_edges, module)?)?;
    Ok(())
}
