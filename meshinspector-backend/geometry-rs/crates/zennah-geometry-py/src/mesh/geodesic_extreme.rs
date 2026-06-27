#[pyfunction]
fn mesh_geodesic_extreme_edges(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    scalars: PyReadonlyArray1<'_, f64>,
    extreme_type: &str,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_scalars = read_f64_values(scalars);
    let rust_extreme_type = match extreme_type {
        "ridge" => zennah_geometry_core::MeshExtremeEdgeType::Ridge,
        "gorge" => zennah_geometry_core::MeshExtremeEdgeType::Gorge,
        _ => {
            return Err(PyValueError::new_err(
                "extreme_type must be 'ridge' or 'gorge'",
            ))
        }
    };
    let result = py
        .detach(|| {
            zennah_geometry_core::mesh_geodesic_extreme_edges(
                &rust_vertices,
                &rust_faces,
                &rust_scalars,
                rust_extreme_type,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item(
        "extreme_type",
        match result.extreme_type {
            zennah_geometry_core::MeshExtremeEdgeType::Ridge => "ridge",
            zennah_geometry_core::MeshExtremeEdgeType::Gorge => "gorge",
        },
    )?;
    output.set_item(
        "edge_indices",
        result
            .edge_indices
            .into_iter()
            .map(|edge| edge.into_iter().map(|index| index as i64).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("meshlib_reference", result.meshlib_reference)?;
    Ok(output.unbind())
}
