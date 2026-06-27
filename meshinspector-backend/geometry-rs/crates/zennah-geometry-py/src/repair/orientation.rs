#[pyfunction]
fn flip_normals(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let flipped_faces = py
        .detach(|| zennah_geometry_core::flip_normals(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output_faces: Vec<i64> = flipped_faces.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("faces", output_faces.into_pyarray(py))?;
    Ok(output.unbind())
}
