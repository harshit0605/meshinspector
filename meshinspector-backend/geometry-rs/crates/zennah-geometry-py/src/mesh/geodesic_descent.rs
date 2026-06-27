#[pyfunction]
fn mesh_steepest_descent_triangle_step(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    vertex_scalars: PyReadonlyArray1<'_, f64>,
    face_index: usize,
    start_barycentric: PyReadonlyArray1<'_, f64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_scalars = read_f64_values(vertex_scalars);
    let rust_start = read_barycentric(start_barycentric)?;
    let step = py
        .detach(|| {
            zennah_geometry_core::mesh_steepest_descent_triangle_step(
                &rust_vertices,
                &rust_faces,
                &rust_scalars,
                face_index,
                rust_start,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("face_index", step.face_index)?;
    output.set_item("start_barycentric", step.start_barycentric.to_vec())?;
    output.set_item("start_point", step.start_point.to_vec())?;
    output.set_item("start_value", step.start_value)?;
    output.set_item("gradient", step.gradient.to_vec())?;
    output.set_item("gradient_norm", step.gradient_norm)?;
    match step.crossed_edge {
        Some(edge) => output.set_item(
            "crossed_edge",
            edge.into_iter().map(|index| index as i64).collect::<Vec<_>>(),
        )?,
        None => output.set_item("crossed_edge", py.None())?,
    }
    match step.edge_position {
        Some(position) => output.set_item("edge_position", position)?,
        None => output.set_item("edge_position", py.None())?,
    }
    match step.crossing_point {
        Some(point) => output.set_item("crossing_point", point.to_vec())?,
        None => output.set_item("crossing_point", py.None())?,
    }
    output.set_item("kind", step.kind)?;
    output.set_item("meshlib_reference", step.meshlib_reference)?;
    Ok(output.unbind())
}

#[pyfunction]
fn mesh_steepest_descent_edge_step(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    vertex_scalars: PyReadonlyArray1<'_, f64>,
    edge: PyReadonlyArray1<'_, i64>,
    edge_position: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_scalars = read_f64_values(vertex_scalars);
    let rust_edge = read_edge("edge", edge)?;
    let step = py
        .detach(|| {
            zennah_geometry_core::mesh_steepest_descent_edge_step(
                &rust_vertices,
                &rust_faces,
                &rust_scalars,
                rust_edge,
                edge_position,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item(
        "start_edge",
        step.start_edge
            .into_iter()
            .map(|index| index as i64)
            .collect::<Vec<_>>(),
    )?;
    output.set_item("edge_position", step.edge_position)?;
    output.set_item("start_point", step.start_point.to_vec())?;
    output.set_item("start_value", step.start_value)?;
    match step.crossed_edge {
        Some(edge) => output.set_item(
            "crossed_edge",
            edge.into_iter().map(|index| index as i64).collect::<Vec<_>>(),
        )?,
        None => output.set_item("crossed_edge", py.None())?,
    }
    match step.crossing_edge_position {
        Some(position) => output.set_item("crossing_edge_position", position)?,
        None => output.set_item("crossing_edge_position", py.None())?,
    }
    match step.crossing_point {
        Some(point) => output.set_item("crossing_point", point.to_vec())?,
        None => output.set_item("crossing_point", py.None())?,
    }
    output.set_item("kind", step.kind)?;
    output.set_item("side", step.side)?;
    output.set_item("meshlib_reference", step.meshlib_reference)?;
    Ok(output.unbind())
}

#[pyfunction]
fn mesh_steepest_descent_vertex_step(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    vertex_scalars: PyReadonlyArray1<'_, f64>,
    vertex_index: usize,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_scalars = read_f64_values(vertex_scalars);
    let step = py
        .detach(|| {
            zennah_geometry_core::mesh_steepest_descent_vertex_step(
                &rust_vertices,
                &rust_faces,
                &rust_scalars,
                vertex_index,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("start_vertex", step.start_vertex)?;
    output.set_item("start_point", step.start_point.to_vec())?;
    output.set_item("start_value", step.start_value)?;
    match step.crossed_edge {
        Some(edge) => output.set_item(
            "crossed_edge",
            edge.into_iter().map(|index| index as i64).collect::<Vec<_>>(),
        )?,
        None => output.set_item("crossed_edge", py.None())?,
    }
    match step.edge_position {
        Some(position) => output.set_item("edge_position", position)?,
        None => output.set_item("edge_position", py.None())?,
    }
    match step.crossing_point {
        Some(point) => output.set_item("crossing_point", point.to_vec())?,
        None => output.set_item("crossing_point", py.None())?,
    }
    match step.gradient_norm {
        Some(norm) => output.set_item("gradient_norm", norm)?,
        None => output.set_item("gradient_norm", py.None())?,
    }
    output.set_item("kind", step.kind)?;
    output.set_item("source", step.source)?;
    output.set_item("meshlib_reference", step.meshlib_reference)?;
    Ok(output.unbind())
}

#[pyfunction]
fn mesh_steepest_descent_path(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    vertex_scalars: PyReadonlyArray1<'_, f64>,
    face_index: usize,
    start_barycentric: PyReadonlyArray1<'_, f64>,
    max_steps: usize,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_scalars = read_f64_values(vertex_scalars);
    let rust_start = read_barycentric(start_barycentric)?;
    let path = py
        .detach(|| {
            zennah_geometry_core::mesh_steepest_descent_path(
                &rust_vertices,
                &rust_faces,
                &rust_scalars,
                face_index,
                rust_start,
                max_steps,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("start_face_index", path.start_face_index)?;
    output.set_item("start_barycentric", path.start_barycentric.to_vec())?;
    output.set_item("start_point", path.start_point.to_vec())?;
    output.set_item("start_value", path.start_value)?;
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
    output.set_item(
        "reached_vertex",
        path.reached_vertex.map(|vertex| vertex as i64),
    )?;
    output.set_item("stopped_reason", path.stopped_reason)?;
    output.set_item("steps", path.steps)?;
    output.set_item("meshlib_reference", path.meshlib_reference)?;
    Ok(output.unbind())
}

fn read_barycentric(values: PyReadonlyArray1<'_, f64>) -> PyResult<[f64; 3]> {
    read_vec3("start_barycentric", values)
}

fn read_edge(name: &str, values: PyReadonlyArray1<'_, i64>) -> PyResult<[i64; 2]> {
    let rows = values.as_array();
    if rows.ndim() != 1 || rows.shape()[0] != 2 {
        return Err(PyValueError::new_err(format!(
            "{name} must have shape (2,)"
        )));
    }
    Ok([rows[0], rows[1]])
}
