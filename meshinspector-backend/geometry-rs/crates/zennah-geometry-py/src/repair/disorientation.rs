fn parse_find_disorientation_ray_mode(
    ray_mode: &str,
) -> PyResult<zennah_geometry_core::FindDisorientationRayMode> {
    match ray_mode {
        "positive" | "Positive" => Ok(zennah_geometry_core::FindDisorientationRayMode::Positive),
        "shallowest" | "Shallowest" => {
            Ok(zennah_geometry_core::FindDisorientationRayMode::Shallowest)
        }
        "both" | "Both" => Ok(zennah_geometry_core::FindDisorientationRayMode::Both),
        _ => Err(PyValueError::new_err(
            "ray_mode must be one of 'positive', 'shallowest', or 'both'",
        )),
    }
}

#[pyfunction(signature = (vertices, faces, ray_mode = "shallowest", epsilon = 1e-8))]
fn find_disoriented_faces(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    ray_mode: &str,
    epsilon: f64,
) -> PyResult<Vec<usize>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let mode = parse_find_disorientation_ray_mode(ray_mode)?;
    py.detach(|| {
        zennah_geometry_core::find_disoriented_faces(&rust_vertices, &rust_faces, mode, epsilon)
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}
