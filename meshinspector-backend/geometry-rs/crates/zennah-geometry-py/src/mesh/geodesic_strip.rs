#[pyfunction]
fn mesh_planar_triangle_strip_path(
    py: Python<'_>,
    start: PyReadonlyArray1<'_, f64>,
    portals: PyReadonlyArray2<'_, f64>,
    end: PyReadonlyArray1<'_, f64>,
) -> PyResult<Py<PyDict>> {
    let start = read_vec2("start", start)?;
    let end = read_vec2("end", end)?;
    let portals = read_portals(portals)?;
    let path = py
        .detach(|| zennah_geometry_core::mesh_planar_triangle_strip_path(start, &portals, end))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("crossing_positions", path.crossing_positions)?;
    output.set_item("crossing_points", vec2_lists(path.crossing_points))?;
    output.set_item("points", vec2_lists(path.points))?;
    output.set_item("segment_lengths", path.segment_lengths)?;
    output.set_item("length_mm", path.length_mm)?;
    output.set_item("meshlib_reference", path.meshlib_reference)?;
    Ok(output.unbind())
}

fn read_vec2(name: &str, values: PyReadonlyArray1<'_, f64>) -> PyResult<[f64; 2]> {
    let rows = values.as_array();
    if rows.ndim() != 1 || rows.shape()[0] != 2 {
        return Err(PyValueError::new_err(format!(
            "{name} must have shape (2,)"
        )));
    }
    Ok([rows[0], rows[1]])
}

fn read_portals(values: PyReadonlyArray2<'_, f64>) -> PyResult<Vec<[[f64; 2]; 2]>> {
    let rows = values.as_array();
    if rows.ndim() != 2 || rows.shape()[1] != 4 {
        return Err(PyValueError::new_err(
            "portals must have shape (n, 4): left_x, left_y, right_x, right_y",
        ));
    }
    let mut portals = Vec::with_capacity(rows.shape()[0]);
    for row in rows.outer_iter() {
        portals.push([[row[0], row[1]], [row[2], row[3]]]);
    }
    Ok(portals)
}

fn vec2_lists(values: Vec<[f64; 2]>) -> Vec<Vec<f64>> {
    values.into_iter().map(|value| value.to_vec()).collect()
}
