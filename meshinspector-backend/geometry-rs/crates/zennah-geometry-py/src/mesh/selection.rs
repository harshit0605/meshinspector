#[pyfunction(signature = (vertices, faces, camera_direction, min_dot = 0.0))]
fn select_camera_facing_faces(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    camera_direction: PyReadonlyArray1<'_, f64>,
    min_dot: f64,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_camera_direction = read_vec3("camera_direction", camera_direction)?;
    py.detach(|| {
        zennah_geometry_core::select_camera_facing_faces(
            &rust_vertices,
            &rust_faces,
            rust_camera_direction,
            min_dot,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction(signature = (vertices, faces, axis, layer_height_mm, max_overhang_distance_mm, hops = 0))]
fn select_overhang_faces(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    axis: PyReadonlyArray1<'_, f64>,
    layer_height_mm: f64,
    max_overhang_distance_mm: f64,
    hops: i64,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_axis = read_vec3("axis", axis)?;
    py.detach(|| {
        zennah_geometry_core::select_overhang_faces(
            &rust_vertices,
            &rust_faces,
            rust_axis,
            layer_height_mm,
            max_overhang_distance_mm,
            hops,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction(signature = (vertices, faces, epsilon = 1e-8))]
fn select_outer_layer_faces(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    epsilon: f64,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| {
        zennah_geometry_core::select_outer_layer_faces(&rust_vertices, &rust_faces, epsilon)
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction(signature = (vertices, faces, source_face_ids, sink_face_ids, boundary_weight = 1.0, curvature_preference = "geodesic"))]
fn graph_cut_select_region(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    source_face_ids: PyReadonlyArray1<'_, i64>,
    sink_face_ids: PyReadonlyArray1<'_, i64>,
    boundary_weight: f64,
    curvature_preference: &str,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_source_face_ids = read_nonnegative_face_ids("source_face_ids", source_face_ids)?;
    let rust_sink_face_ids = read_nonnegative_face_ids("sink_face_ids", sink_face_ids)?;
    py.detach(|| {
        zennah_geometry_core::graph_cut_select_region_with_curvature_preference(
            &rust_vertices,
            &rust_faces,
            &rust_source_face_ids,
            &rust_sink_face_ids,
            boundary_weight,
            curvature_preference,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction(signature = (vertices, faces, source_face_ids, uncertainty_distance_mm, boundary_weight = 1.0, curvature_preference = "geodesic"))]
fn graph_cut_select_region_auto_not_region(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    source_face_ids: PyReadonlyArray1<'_, i64>,
    uncertainty_distance_mm: f64,
    boundary_weight: f64,
    curvature_preference: &str,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_source_face_ids = read_nonnegative_face_ids("source_face_ids", source_face_ids)?;
    py.detach(|| {
        zennah_geometry_core::graph_cut_select_region_auto_not_region_with_curvature_preference(
            &rust_vertices,
            &rust_faces,
            &rust_source_face_ids,
            uncertainty_distance_mm,
            boundary_weight,
            curvature_preference,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction(signature = (vertices, faces, max_dist_sq = 1e-10, max_normal_dot = -0.99, min_area_fraction = 1e-5))]
fn select_overlapping_faces(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    max_dist_sq: f64,
    max_normal_dot: f64,
    min_area_fraction: f64,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| {
        zennah_geometry_core::select_overlapping_faces(
            &rust_vertices,
            &rust_faces,
            max_dist_sq,
            max_normal_dot,
            min_area_fraction,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction(signature = (vertices, faces, area, scalar_type = "absolute", compare_type = "less"))]
fn select_faces_by_area(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    area: f64,
    scalar_type: &str,
    compare_type: &str,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| {
        zennah_geometry_core::select_faces_by_area(
            &rust_vertices,
            &rust_faces,
            area,
            scalar_type,
            compare_type,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn face_adjacency(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<Vec<i64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| zennah_geometry_core::face_adjacency_for_mesh(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn connected_face_components(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<Vec<i64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| {
        zennah_geometry_core::connected_face_components_for_mesh(&rust_vertices, &rust_faces)
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction(signature = (vertices, faces, min_area_mm2 = 0.0))]
fn select_largest_component_faces(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    min_area_mm2: f64,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    py.detach(|| {
        zennah_geometry_core::select_largest_component_faces(
            &rust_vertices,
            &rust_faces,
            min_area_mm2,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn expand_face_selection_to_components(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    seed_face_ids: PyReadonlyArray1<'_, i64>,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let mut rust_seed_face_ids = Vec::new();
    for seed in read_i64_values(seed_face_ids) {
        if seed < 0 {
            return Err(PyValueError::new_err("seed face ids must be non-negative"));
        }
        rust_seed_face_ids.push(seed as usize);
    }
    py.detach(|| {
        zennah_geometry_core::expand_face_selection_to_components(
            &rust_vertices,
            &rust_faces,
            &rust_seed_face_ids,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction(signature = (current_ids, incoming_ids, mode, item_count = None))]
fn apply_meshlib_selection_modifier(
    py: Python<'_>,
    current_ids: PyReadonlyArray1<'_, i64>,
    incoming_ids: PyReadonlyArray1<'_, i64>,
    mode: &str,
    item_count: Option<i64>,
) -> PyResult<Vec<i64>> {
    let current = read_nonnegative_face_ids("current_ids", current_ids)?;
    let incoming = read_nonnegative_face_ids("incoming_ids", incoming_ids)?;
    let item_count = match item_count {
        Some(value) if value < 0 => {
            return Err(PyValueError::new_err("item_count must be non-negative"));
        }
        Some(value) => Some(value as usize),
        None => None,
    };
    py.detach(|| {
        zennah_geometry_core::apply_meshlib_selection_modifier(
            &current,
            &incoming,
            mode,
            item_count,
        )
    })
    .map(|values| values.into_iter().map(|value| value as i64).collect())
    .map_err(|error| PyValueError::new_err(error.to_string()))
}
