use crate::convert::{read_faces, read_points, read_vertices};
use numpy::{IntoPyArray, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

#[pyfunction(signature = (points, mesh_vertices, mesh_faces, up_dist_limit_sq = f64::MAX, lo_dist_limit_sq = 0.0, point_transform = None, mesh_transform = None, face_mask = None))]
fn point_cloud_project_to_mesh(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    mesh_vertices: PyReadonlyArray2<'_, f64>,
    mesh_faces: PyReadonlyArray2<'_, i64>,
    up_dist_limit_sq: f64,
    lo_dist_limit_sq: f64,
    point_transform: Option<Vec<f64>>,
    mesh_transform: Option<Vec<f64>>,
    face_mask: Option<Vec<bool>>,
) -> PyResult<Py<PyDict>> {
    let rust_point_transform = read_optional_transform("point_transform", point_transform)?;
    let rust_mesh_transform = read_optional_transform("mesh_transform", mesh_transform)?;
    let rust_points = read_points(points)?;
    let rust_vertices = read_vertices(mesh_vertices)?;
    let rust_faces = read_faces(mesh_faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::point_cloud_project_to_mesh(
                &rust_points,
                &rust_vertices,
                &rust_faces,
                up_dist_limit_sq,
                lo_dist_limit_sq,
                rust_point_transform,
                rust_mesh_transform,
                face_mask.as_deref(),
            )
        })
        .map_err(PyValueError::new_err)?;

    let output = PyDict::new(py);
    output.set_item(
        "points",
        result
            .points
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .into_pyarray(py),
    )?;
    output.set_item(
        "squared_distances",
        result.squared_distances.into_pyarray(py),
    )?;
    output.set_item("face_indices", result.face_indices.into_pyarray(py))?;
    output.set_item("vertex_indices", result.vertex_indices.into_pyarray(py))?;
    output.set_item(
        "normals",
        result
            .normals
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .into_pyarray(py),
    )?;
    output.set_item("boundary_flags", result.boundary_flags.into_pyarray(py))?;
    Ok(output.unbind())
}

fn read_optional_transform(name: &str, values: Option<Vec<f64>>) -> PyResult<Option<[f64; 16]>> {
    let Some(values) = values else {
        return Ok(None);
    };
    if values.len() != 16 {
        return Err(PyValueError::new_err(format!(
            "{name} must contain 16 row-major values"
        )));
    }
    let mut transform = [0.0; 16];
    for (index, value) in values.into_iter().enumerate() {
        if !value.is_finite() {
            return Err(PyValueError::new_err(format!(
                "{name} values must be finite"
            )));
        }
        transform[index] = value;
    }
    Ok(Some(transform))
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(point_cloud_project_to_mesh, module)?)?;
    Ok(())
}
