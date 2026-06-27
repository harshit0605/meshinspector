#[pyfunction]
fn point_cloud_from_ply(py: Python<'_>, source: &[u8]) -> PyResult<Py<PyDict>> {
    let document = py
        .detach(|| zennah_geometry_core::point_cloud_from_ply(source))
        .map_err(PyValueError::new_err)?;
    point_cloud_ply_to_dict(py, document)
}

#[pyfunction]
fn point_cloud_to_ply(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    normals: Option<PyReadonlyArray2<'_, f64>>,
    colors: Option<PyReadonlyArray2<'_, u8>>,
) -> PyResult<Py<pyo3::types::PyBytes>> {
    let rust_points = read_points(points)?;
    let rust_normals = match normals {
        Some(normals) => Some(read_points(normals)?),
        None => None,
    };
    let rust_colors = match colors {
        Some(colors) => Some(read_point_cloud_colors(colors)?),
        None => None,
    };
    let bytes = py
        .detach(|| {
            zennah_geometry_core::point_cloud_to_ply(
                &rust_points,
                rust_normals.as_deref(),
                rust_colors.as_deref(),
            )
        })
        .map_err(PyValueError::new_err)?;
    Ok(pyo3::types::PyBytes::new(py, &bytes).unbind())
}

fn point_cloud_ply_to_dict(
    py: Python<'_>,
    document: zennah_geometry_core::PointCloudPlyDocument,
) -> PyResult<Py<PyDict>> {
    let points = document.points.into_iter().flatten().collect::<Vec<_>>();
    let normals = document.normals.into_iter().flatten().collect::<Vec<_>>();
    let colors = document.colors.into_iter().flatten().collect::<Vec<_>>();
    let output = PyDict::new(py);
    output.set_item("points", points.into_pyarray(py))?;
    output.set_item("normals", normals.into_pyarray(py))?;
    output.set_item("colors", colors.into_pyarray(py))?;
    Ok(output.unbind())
}

fn read_point_cloud_colors(colors: PyReadonlyArray2<'_, u8>) -> PyResult<Vec<[u8; 3]>> {
    let rows = colors.as_array();
    if rows.ndim() != 2 || rows.shape()[1] != 3 {
        return Err(PyValueError::new_err("colors must have shape (n, 3)"));
    }
    let mut output = Vec::with_capacity(rows.shape()[0]);
    for row in rows.outer_iter() {
        output.push([row[0], row[1], row[2]]);
    }
    Ok(output)
}
