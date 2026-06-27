#[pyfunction]
fn point_cloud_extract_selected_points_as_object(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    selected_point_ids: PyReadonlyArray1<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_points = read_points(points)?;
    let selected = read_i64_values(selected_point_ids)
        .into_iter()
        .map(|index| {
            usize::try_from(index)
                .map_err(|_| PyValueError::new_err("selected_point_ids must be non-negative"))
        })
        .collect::<PyResult<Vec<_>>>()?;
    let result = py
        .detach(|| {
            zennah_geometry_core::point_cloud_extract_selected_points_as_object(
                &rust_points,
                &selected,
            )
        })
        .map_err(PyValueError::new_err)?;
    let points = result.points.into_iter().flatten().collect::<Vec<_>>();
    let source_point_indices = result
        .source_point_indices
        .into_iter()
        .map(|index| index as i64)
        .collect::<Vec<_>>();
    let output = PyDict::new(py);
    output.set_item("points", points.into_pyarray(py))?;
    output.set_item(
        "source_point_indices",
        source_point_indices.into_pyarray(py),
    )?;
    Ok(output.unbind())
}

#[pyfunction(signature = (points, center_index, radius, num_neighbors = 0, boundary_angle = std::f64::consts::PI * 0.9, max_removes = 0, crit_angle = std::f64::consts::TAU, normals = None, untrusted_indices = None))]
fn point_cloud_local_neighbor_fan(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    center_index: usize,
    radius: f64,
    num_neighbors: usize,
    boundary_angle: f64,
    max_removes: usize,
    crit_angle: f64,
    normals: Option<PyReadonlyArray2<'_, f64>>,
    untrusted_indices: Option<PyReadonlyArray1<'_, i64>>,
) -> PyResult<Py<PyDict>> {
    let (rust_points, rust_normals, rust_untrusted_indices) =
        read_point_cloud_inputs(points, normals, untrusted_indices)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::point_cloud_local_neighbor_fan(
                &rust_points,
                center_index,
                radius,
                num_neighbors,
                boundary_angle,
                max_removes,
                crit_angle,
                rust_normals.as_deref(),
                &rust_untrusted_indices,
            )
        })
        .map_err(PyValueError::new_err)?;

    let output = PyDict::new(py);
    output.set_item("neighbors", result.neighbors.into_pyarray(py))?;
    output.set_item("boundary_neighbor", result.boundary_neighbor)?;
    output.set_item("actual_radius", result.actual_radius)?;
    output.set_item("removed_count", result.removed_count)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (points, center_index, radius, num_neighbors = 0, boundary_angle = std::f64::consts::PI * 0.9, max_removes = 0, crit_angle = std::f64::consts::TAU, normals = None, untrusted_indices = None))]
fn point_cloud_local_fan_triangles(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    center_index: usize,
    radius: f64,
    num_neighbors: usize,
    boundary_angle: f64,
    max_removes: usize,
    crit_angle: f64,
    normals: Option<PyReadonlyArray2<'_, f64>>,
    untrusted_indices: Option<PyReadonlyArray1<'_, i64>>,
) -> PyResult<Py<PyDict>> {
    let (rust_points, rust_normals, rust_untrusted_indices) =
        read_point_cloud_inputs(points, normals, untrusted_indices)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::point_cloud_local_fan_triangles(
                &rust_points,
                center_index,
                radius,
                num_neighbors,
                boundary_angle,
                max_removes,
                crit_angle,
                rust_normals.as_deref(),
                &rust_untrusted_indices,
            )
        })
        .map_err(PyValueError::new_err)?;

    let triangles = result.triangles.into_iter().flatten().collect::<Vec<_>>();
    let output = PyDict::new(py);
    output.set_item("triangles", triangles.into_pyarray(py))?;
    output.set_item("boundary_neighbor", result.boundary_neighbor)?;
    output.set_item("actual_radius", result.actual_radius)?;
    output.set_item("removed_count", result.removed_count)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (points, radius, num_neighbors = 0, boundary_angle = std::f64::consts::PI * 0.9, max_removes = 0, crit_angle = std::f64::consts::TAU, normals = None, untrusted_indices = None))]
fn point_cloud_local_triangulation_repetitions(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    radius: f64,
    num_neighbors: usize,
    boundary_angle: f64,
    max_removes: usize,
    crit_angle: f64,
    normals: Option<PyReadonlyArray2<'_, f64>>,
    untrusted_indices: Option<PyReadonlyArray1<'_, i64>>,
) -> PyResult<Py<PyDict>> {
    let (rust_points, rust_normals, rust_untrusted_indices) =
        read_point_cloud_inputs(points, normals, untrusted_indices)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::point_cloud_local_triangulation_repetitions(
                &rust_points,
                radius,
                num_neighbors,
                boundary_angle,
                max_removes,
                crit_angle,
                rust_normals.as_deref(),
                &rust_untrusted_indices,
            )
        })
        .map_err(PyValueError::new_err)?;

    let repetition_counts = result
        .repetition_counts
        .into_iter()
        .map(|value| value as i64)
        .collect::<Vec<_>>();
    let repeated_3 = result.repeated_3.into_iter().flatten().collect::<Vec<_>>();
    let repeated_2 = result.repeated_2.into_iter().flatten().collect::<Vec<_>>();
    let output = PyDict::new(py);
    output.set_item("repetition_counts", repetition_counts.into_pyarray(py))?;
    output.set_item("repeated_3", repeated_3.into_pyarray(py))?;
    output.set_item("repeated_2", repeated_2.into_pyarray(py))?;
    Ok(output.unbind())
}

